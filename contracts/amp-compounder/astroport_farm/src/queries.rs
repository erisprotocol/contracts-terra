use std::ops::Div;

use astroport::asset::{Asset, AssetInfoExt};
use cosmwasm_std::{Decimal, Deps, Env, Order, StdResult};
use cw_storage_plus::Bound;
use eris::{
    astroport_farm::{
        ConfigResponse, ExchangeRatesResponse, StateResponse, UserInfo, UserInfoResponse,
    },
    voting_escrow::{DEFAULT_LIMIT, MAX_LIMIT},
};

use crate::{
    constants::DAY,
    state::{CONFIG, EXCHANGE_HISTORY, STATE},
};

/// ## Description
/// Returns contract config
pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    let state = STATE.load(deps.storage)?;
    let lp_token = config.lp_token;
    Ok(ConfigResponse {
        amp_lp_token: state.amp_lp_token.0,
        lp_token,
        owner: config.owner,
        staking_contract: config.staking_contract.0,
        compound_proxy: config.compound_proxy.0,
        controller: config.controller,
        fee: config.fee,
        fee_collector: config.fee_collector,
        base_reward_token: config.base_reward_token,
        deposit_profit_delay_s: config.deposit_profit_delay.seconds,
    })
}

/// ## Description
/// Returns contract state
pub fn query_state(deps: Deps, env: Env, addr: Option<String>) -> StdResult<StateResponse> {
    let state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;

    let lp_token = config.lp_token;
    let total_lp =
        config.staking_contract.query_deposit(&deps.querier, &lp_token, &env.contract.address)?;

    let total_amp_lp = state.total_bond_share;

    let lp_state = config.compound_proxy.query_lp_state(&deps.querier, lp_token.to_string())?;

    let asset_factor = Decimal::from_ratio(total_lp, lp_state.total_share);
    let locked_assets: Vec<Asset> = lp_state
        .assets
        .into_iter()
        .map(|asset| asset.info.with_balance(asset.amount * asset_factor))
        .collect();

    let user_info = addr
        .and_then(|addr| {
            let staker_addr_validated = deps.api.addr_validate(&addr).ok()?;
            Some(staker_addr_validated)
        })
        .and_then(|addr| {
            let user_amp_lp_amount = state.amp_lp_token.query_amount(&deps.querier, addr).ok()?;
            let user_lp_amount = state.calc_bond_amount(total_lp, user_amp_lp_amount);
            Some(UserInfo {
                user_amp_lp_amount,
                user_lp_amount,
            })
        });

    Ok(StateResponse {
        total_lp,
        total_amp_lp,
        exchange_rate: state.calc_exchange_rate(total_lp),
        pair_contract: lp_state.contract_addr,
        locked_assets,
        user_info,
    })
}

/// ## Description
/// Returns reward info for the staker.
pub fn query_user_info(deps: Deps, env: Env, addr: String) -> StdResult<UserInfoResponse> {
    let staker_addr_validated = deps.api.addr_validate(&addr)?;

    let state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;

    let staking_token = config.lp_token;

    let total_lp = config.staking_contract.query_deposit(
        &deps.querier,
        &staking_token,
        &env.contract.address,
    )?;

    let user_amp_lp_amount =
        state.amp_lp_token.query_amount(&deps.querier, staker_addr_validated)?;
    let user_lp_amount = state.calc_bond_amount(total_lp, user_amp_lp_amount);

    Ok(UserInfoResponse {
        total_lp,
        total_amp_lp: state.total_bond_share,
        user_lp_amount,
        user_amp_lp_amount,
    })
}

pub fn query_exchange_rates(
    deps: Deps,
    _env: Env,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<ExchangeRatesResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let end = start_after.map(Bound::exclusive);
    let exchange_rates = EXCHANGE_HISTORY
        .range(deps.storage, None, end, Order::Descending)
        .take(limit)
        .collect::<StdResult<Vec<(u64, Decimal)>>>()?;

    let apr: Option<Decimal> = if exchange_rates.len() > 1 {
        let current = exchange_rates[0];
        let last = exchange_rates[exchange_rates.len() - 1];

        let delta_time_s = current.0 - last.0;
        let delta_rate = current.1.checked_sub(last.1).unwrap_or_default();

        Some(delta_rate.checked_mul(Decimal::from_ratio(DAY, delta_time_s).div(last.1))?)
    } else {
        None
    };

    Ok(ExchangeRatesResponse {
        exchange_rates,
        apr,
    })
}
