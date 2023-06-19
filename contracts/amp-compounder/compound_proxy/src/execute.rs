use crate::constants::COMMISSION_DENOM;
use crate::error::ContractError;
use crate::state::{validate_slippage, State};
use std::collections::{HashMap, HashSet};
use std::convert::TryInto;

use cosmwasm_std::{
    Addr, Coin, CosmosMsg, Decimal, Decimal256, DepsMut, Env, Isqrt, MessageInfo, QuerierWrapper,
    Response, StdError, StdResult, Uint128, Uint256,
};
use cw20::Expiration;
use eris::adapters::factory::Factory;
use eris::compound_proxy::{CallbackMsg, ExecuteMsg, LpConfig, PairType};

use astroport::asset::{Asset, AssetInfo, AssetInfoExt};
use eris::adapters::asset::AssetEx;
use eris::adapters::pair::Pair;
use eris::helper::assert_uniq_assets;

/// ## Description
/// Performs rewards compounding to LP token. Sender must do token approval upon calling this function.
#[allow(clippy::too_many_arguments)]
pub fn compound(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    rewards: Vec<Asset>,
    receiver: Addr,
    no_swap: Option<bool>,
    slippage_tolerance: Option<Decimal>,
    lp_token: String,
) -> Result<Response, ContractError> {
    assert_uniq_assets(&rewards)?;

    let state = State::default();
    let factory: Option<Factory> = state.config.load(deps.storage)?.factory;
    let lp_config = state
        .lps
        .load(deps.storage, lp_token.clone())
        .map_err(|_| StdError::generic_err(format!("did not find lp {}", lp_token)))?;

    let no_swap = no_swap.unwrap_or(false);

    let mut messages: Vec<CosmosMsg> = vec![];
    let mut native_reward_map: HashMap<AssetInfo, Uint128> = HashMap::new();
    let max_spread = state.get_default_max_spread(deps.storage);
    // Swap reward to asset in the pair
    for reward in rewards {
        reward.deposit_asset(&info, &env.contract.address, &mut messages)?;

        if lp_config.pair_info.asset_infos.contains(&reward.info) {
            // if it is already one of the target assets, let optimal swap handle it
        } else {
            let key = (reward.info.as_bytes(), lp_config.wanted_token.as_bytes());
            let route_config = state.routes.load(deps.storage, key);

            if let Ok(route_config) = route_config {
                messages.push(route_config.create_swap(&reward, max_spread, None)?);
            } else if let Some(factory) = &factory {
                // if factory is set, allowed to query pairs from factory
                messages.push(factory.create_swap(
                    &deps.querier,
                    &reward,
                    &lp_config.wanted_token,
                    max_spread,
                    None,
                )?);
            } else {
                return Err(StdError::generic_err(format!(
                    "did not find route {0}-{1}",
                    reward.info.clone(),
                    lp_config.wanted_token
                ))
                .into());
            }
        }

        if reward.is_native_token() {
            native_reward_map.insert(reward.info, reward.amount);
        }
    }

    if !no_swap {
        messages.push(
            CallbackMsg::OptimalSwap {
                lp_token: lp_token.clone(),
            }
            .into_cosmos_msg(&env.contract.address)?,
        );
    }

    let assets = lp_config.pair_info.query_pools(&deps.querier, env.contract.address.clone())?;
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
            lp_token,
        }
        .into_cosmos_msg(&env.contract.address)?,
    );

    Ok(Response::new().add_messages(messages).add_attribute("action", "ampc/compound"))
}

#[allow(clippy::too_many_arguments)]
pub fn multi_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    into: AssetInfo,
    rewards: Vec<Asset>,
    receiver: Addr,
) -> Result<Response, ContractError> {
    assert_uniq_assets(&rewards)?;

    let state = State::default();
    let factory: Option<Factory> = state.config.load(deps.storage)?.factory;

    let mut messages: Vec<CosmosMsg> = vec![];

    let wanted_token = into;

    let mut send_back = false;
    let max_spread = state.get_default_max_spread(deps.storage);

    // Swap reward to asset in the pair
    for reward in rewards {
        reward.deposit_asset(&info, &env.contract.address, &mut messages)?;

        if reward.info == wanted_token {
            // if it is already the target assets, do nothing and send back
            send_back = true;
        } else {
            let key = (reward.info.as_bytes(), wanted_token.as_bytes());
            let route_config = state.routes.load(deps.storage, key);

            if let Ok(route_config) = route_config {
                messages.push(route_config.create_swap(
                    &reward,
                    max_spread,
                    Some(receiver.clone()),
                )?);
            } else if let Some(factory) = &factory {
                // if factory is set, allowed to query pairs from factory
                messages.push(factory.create_swap(
                    &deps.querier,
                    &reward,
                    &wanted_token,
                    max_spread,
                    Some(receiver.to_string()),
                )?);
            } else {
                return Err(StdError::generic_err(format!(
                    "did not find route {0}-{1}",
                    reward.info.clone(),
                    wanted_token
                ))
                .into());
            }
        }
    }

    if send_back {
        messages.push(
            CallbackMsg::SendSwapResult {
                token: wanted_token,
                receiver: receiver.to_string(),
            }
            .into_cosmos_msg(&env.contract.address)?,
        );
    }

    Ok(Response::new().add_messages(messages).add_attribute("action", "ampc/multi_swap"))
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
        CallbackMsg::OptimalSwap {
            lp_token: lp_addr,
        } => optimal_swap(deps, env, info, lp_addr),
        CallbackMsg::ProvideLiquidity {
            prev_balances,
            slippage_tolerance,
            receiver,
            lp_token: lp_addr,
        } => {
            provide_liquidity(deps, env, info, prev_balances, receiver, slippage_tolerance, lp_addr)
        },
        CallbackMsg::SendSwapResult {
            token,
            receiver,
        } => send_swap_result(deps, env, info, token, receiver),
    }
}

/// # Description
/// Performs optimal swap of assets in the pair contract.
fn optimal_swap(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    lp_addr: String,
) -> Result<Response, ContractError> {
    let state = State::default();
    let lp_config = state.lps.load(deps.storage, lp_addr)?;

    let mut messages: Vec<CosmosMsg> = vec![];

    match lp_config.pair_info.pair_type {
        Some(PairType::Stable {}) => {
            //Do nothing for stable pair
        },
        _ => {
            let assets = lp_config.pair_info.query_pools(&deps.querier, env.contract.address)?;
            let asset_a = assets[0].clone();
            let asset_b = assets[1].clone();
            let max_spread = state.get_default_max_spread(deps.storage);
            if !asset_a.amount.is_zero() || !asset_b.amount.is_zero() {
                calculate_optimal_swap(
                    &deps.querier,
                    &lp_config,
                    asset_a,
                    asset_b,
                    &mut messages,
                    max_spread,
                )?;
            }
        },
    }

    Ok(Response::new().add_messages(messages).add_attribute("action", "ampc/optimal_swap"))
}

// returns the token back to the sender
fn send_swap_result(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    token: AssetInfo,
    receiver: String,
) -> Result<Response, ContractError> {
    let amount = token.query_pool(&deps.querier, env.contract.address)?;
    let return_amount = token.with_balance(amount);

    Ok(Response::new()
        .add_message(return_amount.into_msg(&deps.querier, receiver)?)
        .add_attribute("action", "ampc/send_swap_result"))
}

/// # Description
/// Calculates the amount of asset in the pair contract that need to be swapped before providing liquidity.
/// The swap messages will be added to **messages**.
pub fn calculate_optimal_swap(
    querier: &QuerierWrapper,
    lp_config: &LpConfig,
    asset_a: Asset,
    asset_b: Asset,
    messages: &mut Vec<CosmosMsg>,
    max_spread: Decimal,
) -> StdResult<(Uint128, Uint128, Uint128, Uint128)> {
    let mut swap_asset_a_amount = Uint128::zero();
    let mut swap_asset_b_amount = Uint128::zero();
    let mut return_a_amount = Uint128::zero();
    let mut return_b_amount = Uint128::zero();

    let pair_contract = lp_config.pair_info.contract_addr.clone();
    let pools = lp_config.pair_info.query_pools(querier, pair_contract.clone())?;
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
            lp_config.commission_bps,
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
                Decimal256::from_ratio(lp_config.commission_bps, COMMISSION_DENOM),
            )?;
            if !return_b_amount.is_zero() {
                swap_asset_a_amount = swap_asset.amount;
                messages.push(Pair(pair_contract).swap_msg(
                    &swap_asset,
                    None,
                    Some(max_spread),
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
            lp_config.commission_bps,
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
                Decimal256::from_ratio(lp_config.commission_bps, COMMISSION_DENOM),
            )?;
            if !return_a_amount.is_zero() {
                swap_asset_b_amount = swap_asset.amount;
                messages.push(Pair(pair_contract).swap_msg(
                    &swap_asset,
                    None,
                    Some(max_spread),
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
    lp_addr: String,
) -> Result<Response, ContractError> {
    let state = State::default();
    let lp_config = state.lps.load(deps.storage, lp_addr)?;

    let pair_contract = lp_config.pair_info.contract_addr.clone();

    let assets = lp_config.pair_info.query_pools(&deps.querier, env.contract.address)?;

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
        Some(slippage_tolerance.unwrap_or(lp_config.slippage_tolerance)),
        Some(receiver.to_string()),
        funds,
    )?;
    messages.push(provide_liquidity);

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "ampc/provide_liquidity")
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

pub fn update_config(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            factory,
            remove_factory,
            upsert_lps,
            delete_lps,
            insert_routes,
            delete_routes,
            default_max_spread: default_slippage,
        } => {
            let state = State::default();

            state.assert_owner(deps.storage, &info.sender)?;

            if let Some(factory) = factory {
                let factory = Some(Factory(deps.api.addr_validate(&factory)?));
                state.config.update::<_, StdError>(deps.storage, |mut config| {
                    config.factory = factory;
                    Ok(config)
                })?;
            } else if let Some(remove_factory) = remove_factory {
                if remove_factory {
                    state.config.update::<_, StdError>(deps.storage, |mut config| {
                        config.factory = None;
                        Ok(config)
                    })?;
                }
            }

            if let Some(removed_lps) = delete_lps {
                for removed_lp in removed_lps {
                    state.remove_lp(&mut deps, removed_lp)?;
                }
            }

            if let Some(added_lps) = upsert_lps {
                let mut used_pairs = HashSet::new();
                for added_lp in added_lps {
                    if !used_pairs.insert(added_lp.pair_contract.to_string()) {
                        return Err(ContractError::AddPairContractDuplicated(
                            added_lp.pair_contract,
                        ));
                    }
                    state.add_lp(&mut deps, added_lp)?;
                }
            }

            if let Some(delete_routes) = delete_routes {
                for delete_route in delete_routes {
                    state.delete_route(
                        &mut deps,
                        (delete_route.from, delete_route.to),
                        delete_route.both.unwrap_or(true),
                    )?;
                }
            }

            if let Some(added_routes) = insert_routes {
                for added_route in added_routes {
                    state.add_route(&mut deps, added_route)?;
                }
            }

            if let Some(default_slippage) = default_slippage {
                validate_slippage(Decimal::percent(default_slippage))?;
                state.default_max_spread.save(deps.storage, &default_slippage)?;
            }

            Ok(Response::new().add_attribute("action", "ampc/update_config"))
        },
        _ => Err(StdError::generic_err("not supported").into()),
    }
}
