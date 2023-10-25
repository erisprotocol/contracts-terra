use crate::constants::{CONTRACT_NAME, CONTRACT_VERSION};
use crate::error::ContractError;
use crate::state::{Config, CONFIG, OWNERSHIP_PROPOSAL};

use astroport::asset::{Asset, AssetInfo, AssetInfoExt};

use astroport::common::{claim_ownership, drop_ownership_proposal, propose_new_owner};
use cosmwasm_std::{
    attr, entry_point, to_binary, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo,
    Order, Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_storage_plus::Map;
use eris::adapters::asset::AssetEx;
use eris::adapters::compounder::Compounder;
use eris::fees_collector::{
    AssetWithLimit, BalancesResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
    TargetConfig,
};
use eris::helper::funds_or_allowance;
use std::cmp;
use std::collections::HashSet;
use std::vec;

/// Sets the default maximum spread (as a percentage) used when swapping fee tokens to stablecoin.
const DEFAULT_MAX_SPREAD: u64 = 5; // 5%

/// ## Description
/// Creates a new contract with the specified parameters in the [`InstantiateMsg`].
/// Returns the [`Response`] with the specified attributes if the operation was successful, or a [`ContractError`] if the contract was not created.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let max_spread = if let Some(max_spread) = msg.max_spread {
        if max_spread.gt(&Decimal::one()) {
            return Err(ContractError::IncorrectMaxSpread {});
        };
        max_spread
    } else {
        Decimal::percent(DEFAULT_MAX_SPREAD)
    };

    msg.stablecoin.check(deps.api)?;

    let config = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
        operator: deps.api.addr_validate(&msg.operator)?,
        factory_contract: deps.api.addr_validate(&msg.factory_contract)?,
        stablecoin: msg.stablecoin,
        target_list: msg
            .target_list
            .into_iter()
            .map(|target| target.validate(deps.api))
            .collect::<StdResult<_>>()?,
        max_spread,
        compound_proxy: deps.api.addr_validate(&msg.zapper)?,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

/// ## Description
/// Exposes execute functions available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Collect {
            assets,
        } => collect(deps, env, info, assets),
        ExecuteMsg::UpdateConfig {
            operator,
            factory_contract,
            target_list,
            max_spread,
            zapper,
        } => update_config(deps, info, operator, factory_contract, target_list, max_spread, zapper),
        ExecuteMsg::DistributeFees {} => distribute_fees(deps, env, info),
        ExecuteMsg::ProposeNewOwner {
            owner,
            expires_in,
        } => {
            let config: Config = CONFIG.load(deps.storage)?;

            propose_new_owner(deps, info, env, owner, expires_in, config.owner, OWNERSHIP_PROPOSAL)
                .map_err(|e| e.into())
        },
        ExecuteMsg::DropOwnershipProposal {} => {
            let config: Config = CONFIG.load(deps.storage)?;

            drop_ownership_proposal(deps, info, config.owner, OWNERSHIP_PROPOSAL)
                .map_err(|e| e.into())
        },
        ExecuteMsg::ClaimOwnership {} => {
            claim_ownership(deps, info, env, OWNERSHIP_PROPOSAL, |deps, new_owner| {
                CONFIG.update::<_, StdError>(deps.storage, |mut v| {
                    v.owner = new_owner;
                    Ok(v)
                })?;

                Ok(())
            })
            .map_err(|e| e.into())
        },
    }
}

/// ## Description
/// Swaps fee tokens to stablecoin and distribute the resulting stablecoin to the target list.
/// Returns a [`ContractError`] on failure, otherwise returns a [`Response`] object if the
/// operation was successful.
fn collect(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    assets: Vec<AssetWithLimit>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let stablecoin = config.stablecoin.clone();

    if info.sender != config.operator {
        return Err(ContractError::Unauthorized {});
    }

    // Check for duplicate assets
    let mut uniq = HashSet::new();
    if !assets.clone().into_iter().all(|a| uniq.insert(a.info.to_string())) {
        return Err(ContractError::DuplicatedAsset {});
    }
    let response = Response::default();

    // Swap all non stablecoin tokens
    let mut messages = swap_assets(
        deps.as_ref(),
        env.clone(),
        &config,
        assets.into_iter().filter(|a| a.info.ne(&stablecoin)).collect(),
    )?;

    let distribute_fee = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::DistributeFees {})?,
        funds: vec![],
    });
    messages.push(distribute_fee);

    Ok(response.add_messages(messages).add_attribute("action", "ampfee/collect"))
}

/// ## Description
/// Swap all non stablecoin tokens to stablecoin. Returns a [`ContractError`] on failure, otherwise returns
/// a [`Response`] object if the operation was successful.
fn swap_assets(
    deps: Deps,
    env: Env,
    config: &Config,
    assets: Vec<AssetWithLimit>,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let balances = to_asset_balances(&deps, &env, assets, &config.stablecoin)?;

    if balances.is_empty() {
        return Ok(vec![]);
    }

    let (funds, mut allowances) =
        funds_or_allowance(&env, &config.compound_proxy, &balances, None)?;
    let multi_swap = Compounder(config.compound_proxy.clone()).multi_swap_msg(
        balances,
        config.stablecoin.clone(),
        funds,
        None,
    )?;

    allowances.push(multi_swap);

    Ok(allowances)
}

fn to_asset_balances(
    deps: &Deps,
    env: &Env,
    assets: Vec<AssetWithLimit>,
    stablecoin: &AssetInfo,
) -> StdResult<Vec<Asset>> {
    let mut result = vec![];
    for asset in assets {
        if asset.info != *stablecoin {
            let mut balance = asset.info.query_pool(&deps.querier, env.contract.address.clone())?;
            if let Some(limit) = asset.limit {
                if limit < balance && limit > Uint128::zero() {
                    balance = limit;
                }
            }
            if !balance.is_zero() {
                result.push(asset.info.with_balance(balance))
            }
        }
    }

    Ok(result)
}
/// ## Description
/// Distributes stablecoin rewards to the target list. Returns a [`ContractError`] on failure.
fn distribute_fees(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    // Only the contract itself can call this function
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized {});
    }

    let config = CONFIG.load(deps.storage)?;
    let (distribute_msg, attributes) = distribute(deps, env, config)?;

    Ok(Response::new().add_messages(distribute_msg).add_attributes(attributes))
}

type DistributeMsgParts = (Vec<CosmosMsg>, Vec<(String, String)>);

/// ## Description
/// Private function that performs the stablecoin token distribution to beneficiary. Returns a [`ContractError`] on failure,
/// otherwise returns a vector that contains the objects of type [`CosmosMsg`] if the operation was successful.
fn distribute(
    deps: DepsMut,
    env: Env,
    config: Config,
) -> Result<DistributeMsgParts, ContractError> {
    let mut messages = vec![];
    let mut attributes = vec![];

    let stablecoin = config.stablecoin.clone();

    let mut total_amount = stablecoin.query_pool(&deps.querier, env.contract.address.clone())?;
    if total_amount.is_zero() {
        return Ok((messages, attributes));
    }

    let mut weighted = vec![];
    for target in config.target_list {
        match target.target_type {
            eris::fees_collector::TargetType::Weight => weighted.push(target),
            eris::fees_collector::TargetType::FillUpFirst {
                filled_to,
                min_fill,
            } => {
                let filled_asset = &stablecoin;
                let current_asset_amount =
                    filled_asset.query_pool(&deps.querier, target.addr.to_string())?;

                if filled_to > current_asset_amount {
                    let amount =
                        cmp::min(total_amount, filled_to.checked_sub(current_asset_amount)?);
                    let min_fill = min_fill.unwrap_or_default();
                    if amount > min_fill {
                        // reduce amount from total_amount. Rest is distributed by share
                        total_amount = total_amount.checked_sub(amount)?;

                        let send_msg = stablecoin.with_balance(amount).transfer_msg_target(
                            &deps.api.addr_validate(target.addr.as_str())?,
                            target.msg,
                        )?;

                        messages.push(send_msg);
                        attributes.push(("type".to_string(), "fill_up".to_string()));
                        attributes.push(("to".to_string(), target.addr.to_string()));
                        attributes.push(("amount".to_string(), amount.to_string()));
                    }
                }
            },
            eris::fees_collector::TargetType::Ibc {
                ..
            } => {
                weighted.push(target);
            },
        }
    }

    let total_weight = weighted.iter().map(|target| target.weight).sum::<u64>();

    for target in weighted {
        let amount = total_amount.multiply_ratio(target.weight, total_weight);
        if !amount.is_zero() {
            match target.target_type {
                eris::fees_collector::TargetType::Weight => {
                    let send_msg = stablecoin.with_balance(amount).transfer_msg_target(
                        &deps.api.addr_validate(target.addr.as_str())?,
                        target.msg,
                    )?;
                    messages.push(send_msg);
                },
                eris::fees_collector::TargetType::Ibc {
                    channel_id,
                    ics20,
                } => {
                    let send_msg = stablecoin.with_balance(amount).transfer_msg_ibc(
                        &env,
                        target.addr.clone(),
                        channel_id,
                        ics20,
                    )?;
                    messages.push(send_msg);
                },
                _ => (),
            }

            attributes.push(("to".to_string(), target.addr.to_string()));
            attributes.push(("amount".to_string(), amount.to_string()));
        }
    }

    attributes.push(("action".to_string(), "ampfee/distribute_fees".to_string()));

    Ok((messages, attributes))
}

/// ## Description
/// Updates contract config. Returns a [`ContractError`] on failure or the [`CONFIG`] data will be updated.
#[allow(clippy::too_many_arguments)]
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    operator: Option<String>,
    factory_contract: Option<String>,
    target_list: Option<Vec<TargetConfig>>,
    max_spread: Option<Decimal>,
    compound_proxy: Option<String>,
) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(operator) = operator {
        config.operator = deps.api.addr_validate(&operator)?;
    }

    if let Some(factory_contract) = factory_contract {
        config.factory_contract = deps.api.addr_validate(&factory_contract)?;
    }

    if let Some(compound_proxy) = compound_proxy {
        config.compound_proxy = deps.api.addr_validate(&compound_proxy)?;
    }

    if let Some(max_spread) = max_spread {
        if max_spread.gt(&Decimal::one()) {
            return Err(ContractError::IncorrectMaxSpread {});
        };
        config.max_spread = max_spread;
    }

    if let Some(target_list) = target_list {
        config.target_list = target_list
            .into_iter()
            .map(|target| target.validate(deps.api))
            .collect::<StdResult<_>>()?
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

/// ## Description
/// Exposes all the queries available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::Balances {
            assets,
        } => to_binary(&query_get_balances(deps, env, assets)?),
    }
}

/// ## Description
/// Returns token balances for specific tokens using a [`ConfigResponse`] object.
fn query_get_balances(deps: Deps, env: Env, assets: Vec<AssetInfo>) -> StdResult<BalancesResponse> {
    let mut resp = BalancesResponse {
        balances: vec![],
    };

    for a in assets {
        // Get balance
        let balance = a.query_pool(&deps.querier, env.contract.address.clone())?;
        if !balance.is_zero() {
            resp.balances.push(Asset {
                info: a,
                amount: balance,
            })
        }
    }

    Ok(resp)
}

/// ## Description
/// Used for contract migration. Returns a default object of type [`Response`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if CONTRACT_VERSION == "2.0.0" {
        let map: Map<String, AssetInfo> = Map::new("bridges");
        let keys: Vec<String> = map
            .keys(deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<String>>>()?;

        for key in keys {
            map.remove(deps.storage, key)
        }
    }

    Ok(Response::new()
        .add_attribute("new_contract_name", CONTRACT_NAME)
        .add_attribute("new_contract_version", CONTRACT_VERSION))
}
