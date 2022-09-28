use crate::error::ContractError;
use crate::simulation::query_compound_simulation;
use crate::state::{Config, CONFIG, PAIR_PROXY};
use std::collections::HashMap;
use std::convert::TryInto;

use astroport::factory::PairType;
use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, Coin, CosmosMsg, Decimal, Decimal256, Deps, DepsMut, Env,
    Isqrt, MessageInfo, QuerierWrapper, Response, StdError, StdResult, Uint128, Uint256,
};
use cw20::Expiration;
use eris::compound_proxy::{CallbackMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

use astroport::asset::{addr_validate_to_lower, Asset, AssetInfo, AssetInfoExt};
use eris::adapters::asset::AssetEx;
use eris::adapters::pair::Pair;

/// Scaling denominator for commission
const COMMISSION_DENOM: u64 = 10000u64;

/// ## Description
/// Validates that commission bps must be less than or equal 10000
fn validate_commission(commission_bps: u64) -> StdResult<u64> {
    if commission_bps >= 10000u64 {
        Err(StdError::generic_err("commission rate must be 0 to 9999"))
    } else {
        Ok(commission_bps)
    }
}

/// ## Description
/// Validates that decimal value is in the range 0 to 1
fn validate_percentage(value: Decimal, field: &str) -> StdResult<Decimal> {
    if value > Decimal::one() {
        Err(StdError::generic_err(field.to_string() + " must be 0 to 1"))
    } else {
        Ok(value)
    }
}

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
    let commission_bps = validate_commission(msg.commission_bps)?;
    let slippage_tolerance = validate_percentage(msg.slippage_tolerance, "slippage_tolerance")?;
    let pair_contract = addr_validate_to_lower(deps.api, msg.pair_contract.as_str())?;
    let pair_info = Pair(pair_contract).query_pair_info(&deps.querier)?;

    let config = Config {
        pair_info,
        commission_bps,
        slippage_tolerance,
    };
    CONFIG.save(deps.storage, &config)?;

    for (asset_info, pair_proxy) in msg.pair_proxies {
        asset_info.check(deps.api)?;
        let pair_proxy_addr = addr_validate_to_lower(deps.api, &pair_proxy)?;
        PAIR_PROXY.save(deps.storage, asset_info.to_string(), &Pair(pair_proxy_addr))?;
    }

    Ok(Response::new())
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
        ExecuteMsg::Compound {
            rewards,
            to,
            no_swap,
            slippage_tolerance,
        } => {
            let to_addr = if let Some(to_addr) = to {
                Some(addr_validate_to_lower(deps.api, &to_addr)?)
            } else {
                None
            };
            compound(
                deps,
                env,
                info.clone(),
                info.sender,
                rewards,
                to_addr,
                no_swap,
                slippage_tolerance,
            )
        },
        ExecuteMsg::Callback(msg) => handle_callback(deps, env, info, msg),
    }
}

/// ## Description
/// Performs rewards compounding to LP token. Sender must do token approval upon calling this function.
#[allow(clippy::too_many_arguments)]
pub fn compound(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: Addr,
    rewards: Vec<Asset>,
    to: Option<Addr>,
    no_swap: Option<bool>,
    slippage_tolerance: Option<Decimal>,
) -> Result<Response, ContractError> {
    let receiver = to.unwrap_or(sender);
    let no_swap = no_swap.unwrap_or(false);

    let mut messages: Vec<CosmosMsg> = vec![];
    let mut native_reward_map: HashMap<AssetInfo, Uint128> = HashMap::new();
    // Swap reward to asset in the pair
    for reward in rewards {
        reward.deposit_asset(&info, &env.contract.address, &mut messages)?;

        let pair_proxy = PAIR_PROXY.may_load(deps.storage, reward.info.to_string())?;
        if let Some(pair_proxy) = pair_proxy {
            let swap_reward =
                pair_proxy.swap_msg(&reward, None, Some(Decimal::percent(50u64)), None)?;
            messages.push(swap_reward);
        }

        if reward.is_native_token() {
            native_reward_map.insert(reward.info, reward.amount);
        }
    }

    if !no_swap {
        messages.push(CallbackMsg::OptimalSwap {}.into_cosmos_msg(&env.contract.address)?);
    }

    let config = CONFIG.load(deps.storage)?;
    let assets = config.pair_info.query_pools(&deps.querier, env.contract.address.clone())?;
    let prev_balances = assets
        .iter()
        .map(|a| {
            let balance = a
                .amount
                .checked_sub(*native_reward_map.get(&a.info).unwrap_or(&Uint128::zero()))?;
            Ok(a.info.with_balance(balance))
        })
        .collect::<StdResult<_>>()?;

    messages.push(
        CallbackMsg::ProvideLiquidity {
            prev_balances,
            slippage_tolerance,
            receiver: receiver.to_string(),
        }
        .into_cosmos_msg(&env.contract.address)?,
    );

    Ok(Response::new().add_messages(messages).add_attribute("action", "compound"))
}

/// # Description
/// Handle the callbacks describes in the [`CallbackMsg`]. Returns an [`ContractError`] on failure, otherwise returns the [`Response`]
pub fn handle_callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: CallbackMsg,
) -> Result<Response, ContractError> {
    // Callback functions can only be called by this contract itself
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized {});
    }
    match msg {
        CallbackMsg::OptimalSwap {} => optimal_swap(deps, env, info),
        CallbackMsg::ProvideLiquidity {
            prev_balances,
            slippage_tolerance,
            receiver,
        } => provide_liquidity(deps, env, info, prev_balances, receiver, slippage_tolerance),
    }
}

/// # Description
/// Performs optimal swap of assets in the pair contract.
fn optimal_swap(deps: DepsMut, env: Env, _info: MessageInfo) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    let mut messages: Vec<CosmosMsg> = vec![];

    match config.pair_info.pair_type {
        PairType::Stable {} => {
            //Do nothing for stable pair
        },
        _ => {
            let assets = config.pair_info.query_pools(&deps.querier, env.contract.address)?;
            let asset_a = assets[0].clone();
            let asset_b = assets[1].clone();
            if !asset_a.amount.is_zero() || !asset_b.amount.is_zero() {
                calculate_optimal_swap(
                    &deps.querier,
                    &config,
                    asset_a,
                    asset_b,
                    None,
                    None,
                    &mut messages,
                )?;
            }
        },
    }

    Ok(Response::new().add_messages(messages).add_attribute("action", "optimal_swap"))
}

/// # Description
/// Calculates the amount of asset in the pair contract that need to be swapped before providing liquidity.
/// The swap messages will be added to **messages**.
pub fn calculate_optimal_swap(
    querier: &QuerierWrapper,
    config: &Config,
    asset_a: Asset,
    asset_b: Asset,
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    messages: &mut Vec<CosmosMsg>,
) -> StdResult<(Uint128, Uint128, Uint128, Uint128)> {
    let mut swap_asset_a_amount = Uint128::zero();
    let mut swap_asset_b_amount = Uint128::zero();
    let mut return_a_amount = Uint128::zero();
    let mut return_b_amount = Uint128::zero();

    let pair_contract = config.pair_info.contract_addr.clone();
    let pools = config.pair_info.query_pools(querier, pair_contract.clone())?;
    let provide_a_amount: Uint256 = asset_a.amount.into();
    let provide_b_amount: Uint256 = asset_b.amount.into();
    let pool_a_amount: Uint256 = pools[0].amount.into();
    let pool_b_amount: Uint256 = pools[1].amount.into();
    let provide_a_area = provide_a_amount * pool_b_amount;
    let provide_b_area = provide_b_amount * pool_a_amount;

    #[allow(clippy::comparison_chain)]
    if provide_a_area > provide_b_area {
        let swap_amount = get_swap_amount(
            provide_a_amount,
            provide_b_amount,
            pool_a_amount,
            pool_b_amount,
            config.commission_bps,
        )?;
        if !swap_amount.is_zero() {
            let swap_asset = Asset {
                info: asset_a.info,
                amount: swap_amount,
            };
            return_b_amount = simulate(
                pool_a_amount,
                pool_b_amount,
                swap_asset.amount.into(),
                Decimal256::from_ratio(config.commission_bps, COMMISSION_DENOM),
            )?;
            if !return_b_amount.is_zero() {
                swap_asset_a_amount = swap_asset.amount;
                messages.push(Pair(pair_contract).swap_msg(
                    &swap_asset,
                    belief_price,
                    max_spread,
                    None,
                )?);
            }
        }
    } else if provide_a_area < provide_b_area {
        let swap_amount = get_swap_amount(
            provide_b_amount,
            provide_a_amount,
            pool_b_amount,
            pool_a_amount,
            config.commission_bps,
        )?;
        if !swap_amount.is_zero() {
            let swap_asset = Asset {
                info: asset_b.info,
                amount: swap_amount,
            };
            return_a_amount = simulate(
                pool_b_amount,
                pool_a_amount,
                swap_asset.amount.into(),
                Decimal256::from_ratio(config.commission_bps, COMMISSION_DENOM),
            )?;
            if !return_a_amount.is_zero() {
                swap_asset_b_amount = swap_asset.amount;
                messages.push(Pair(pair_contract).swap_msg(
                    &swap_asset,
                    belief_price,
                    max_spread,
                    None,
                )?);
            }
        }
    };

    Ok((swap_asset_a_amount, swap_asset_b_amount, return_a_amount, return_b_amount))
}

/// ## Description
/// Provides liquidity on the pair contract to get LP token.
pub fn provide_liquidity(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    prev_balances: Vec<Asset>,
    receiver: String,
    slippage_tolerance: Option<Decimal>,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    let pair_contract = config.pair_info.contract_addr.clone();

    let assets = config.pair_info.query_pools(&deps.querier, env.contract.address)?;

    let prev_balance_map: HashMap<_, _> =
        prev_balances.into_iter().map(|a| (a.info, a.amount)).collect();

    let mut messages: Vec<CosmosMsg> = vec![];
    let mut provide_assets: Vec<Asset> = vec![];
    let mut funds: Vec<Coin> = vec![];
    for asset in assets.iter() {
        let prev_balance = *prev_balance_map.get(&asset.info).unwrap_or(&Uint128::zero());
        let amount = asset.amount.checked_sub(prev_balance)?;
        let provide_asset = asset.info.with_balance(amount);
        provide_assets.push(provide_asset.clone());

        if !provide_asset.amount.is_zero() {
            if asset.is_native_token() {
                funds.push(Coin {
                    denom: provide_asset.info.to_string(),
                    amount: provide_asset.amount,
                });
            } else {
                messages.push(provide_asset.increase_allowance_msg(
                    pair_contract.to_string(),
                    Some(Expiration::AtHeight(env.block.height + 1)),
                )?);
            }
        }
    }

    let provide_liquidity = Pair(pair_contract).provide_liquidity_msg(
        provide_assets,
        Some(slippage_tolerance.unwrap_or(config.slippage_tolerance)),
        Some(receiver.to_string()),
        funds,
    )?;
    messages.push(provide_liquidity);

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "provide_liquidity")
        .add_attribute("receiver", receiver))
}

/// Calculate swap amount
pub(crate) fn get_swap_amount(
    amount_a: Uint256,
    amount_b: Uint256,
    pool_a: Uint256,
    pool_b: Uint256,
    commission_bps: u64,
) -> StdResult<Uint128> {
    let pool_ax = amount_a + pool_a;
    let pool_bx = amount_b + pool_b;
    let area_ax = pool_ax * pool_b;
    let area_bx = pool_bx * pool_a;

    let a = Uint256::from(commission_bps * commission_bps) * area_ax
        + Uint256::from(4u64 * (COMMISSION_DENOM - commission_bps) * COMMISSION_DENOM) * area_bx;
    let b = Uint256::from(commission_bps) * area_ax + area_ax.isqrt() * a.isqrt();
    let result = (b / Uint256::from(2u64 * COMMISSION_DENOM) / pool_bx).saturating_sub(pool_a);

    result.try_into().map_err(|_| StdError::generic_err("overflow"))
}

/// Simulates return amount from the swap
fn simulate(
    offer_pool: Uint256,
    ask_pool: Uint256,
    offer_amount: Uint256,
    commission_rate: Decimal256,
) -> StdResult<Uint128> {
    // offer => ask
    // ask_amount = (ask_pool - cp / (offer_pool + offer_amount)) * (1 - commission_rate)
    let cp: Uint256 = offer_pool * ask_pool;
    let return_amount: Uint256 = (Decimal256::from_ratio(ask_pool, 1u64)
        - Decimal256::from_ratio(cp, offer_pool + offer_amount))
        * Uint256::from(1u64);

    // calculate commission
    let commission_amount: Uint256 = return_amount * commission_rate;

    // commission will be absorbed to pool
    let return_amount: Uint256 = return_amount - commission_amount;

    return_amount.try_into().map_err(|_| StdError::generic_err("overflow"))
}

/// ## Description
/// Exposes all the queries available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::CompoundSimulation {
            rewards,
        } => to_binary(&query_compound_simulation(deps, rewards)?),
    }
}

/// ## Description
/// Used for migration of contract. Returns the default object of type [`Response`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
