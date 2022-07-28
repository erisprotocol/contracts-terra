use cosmwasm_std::{Decimal, Deps, Env, StdResult, Uint128};

use eris_staking::yieldextractor::{ConfigResponse, ShareResponse, StateResponse};
use eris_staking::DecimalCheckedOps;

use crate::helpers::{query_cw20_balance, query_cw20_total_supply, query_exchange_rate};
use crate::math::compute_withdraw_amount;
use crate::state::State;

// const MAX_LIMIT: u32 = 30;
// const DEFAULT_LIMIT: u32 = 10;

pub fn config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = State::default();

    let config = state.extract_config.load(deps.storage)?;

    Ok(ConfigResponse {
        owner: state.owner.load(deps.storage)?.into(),
        new_owner: state.new_owner.may_load(deps.storage)?.map(|addr| addr.into()),
        lp_token: state.lp_token.load(deps.storage)?.into(),
        stake_token: state.stake_token.load(deps.storage)?.into(),

        hub_contract: config.hub_contract.to_string(),
        interface: config.interface,
        yield_extract_addr: config.yield_extract_addr.to_string(),
        yield_extract_p: config.yield_extract_p,
    })
}

pub fn state(deps: Deps, env: Env, addr: Option<String>) -> StdResult<StateResponse> {
    let state = State::default();

    let extract_config = state.extract_config.load(deps.storage)?;

    let lp_token = state.lp_token.load(deps.storage)?;
    let total_lp = query_cw20_total_supply(&deps.querier, &lp_token)?;

    let stake_token = state.stake_token.load(deps.storage)?;
    let stake_balance = query_cw20_balance(&deps.querier, &stake_token, &env.contract.address)?;
    let stake_harvested = state.stake_harvested.load(deps.storage)?;
    let mut stake_extracted = state.stake_extracted.load(deps.storage)?;
    let mut stake_available = stake_balance.checked_sub(stake_extracted)?;

    let last_exchange_rate = state.last_exchange_rate.load(deps.storage)?;
    let exchange_rate_stake_uluna =
        query_exchange_rate(&deps.querier, extract_config.interface, &extract_config.hub_contract)?;

    if exchange_rate_stake_uluna.le(&last_exchange_rate) || last_exchange_rate.is_zero() {
        // if the current rate is lower or equal to the last exchange rate nothing will be extracted
        // it is expected that exchange_rate will only increase - slashings ignored / nothing extracted until it is higher again.
        // if last_exchange_rate is not set, nothing is deposited anyways.
    } else {
        // no check needed, as we checked for "le" already. exchange_rate_stake_uluna is also not zero

        // (20 - 10) / 20 = 0.5
        let exchange_rate_diff =
            (exchange_rate_stake_uluna - last_exchange_rate) / exchange_rate_stake_uluna;

        // 0.5 * 0.1 * 100_000000 = 5_000000
        let stake_to_extract = exchange_rate_diff
            .checked_mul(extract_config.yield_extract_p)?
            .checked_mul_uint(stake_available)?;

        stake_extracted = stake_extracted.checked_add(stake_to_extract)?;
        stake_available = stake_balance.checked_sub(stake_extracted)?;
    }

    let exchange_rate_lp_stake = if total_lp.is_zero() {
        Decimal::zero()
    } else {
        Decimal::from_ratio(stake_available, total_lp)
    };

    let user_share = if let Some(addr) = addr {
        Some(query_cw20_balance(&deps.querier, &lp_token, &deps.api.addr_validate(&addr)?)?)
    } else {
        None
    };

    let user_received_asset =
        user_share.map(|user_share| compute_withdraw_amount(total_lp, user_share, stake_available));

    Ok(StateResponse {
        total_lp,
        stake_balance,
        stake_extracted,
        stake_harvested,
        stake_available,

        exchange_rate_lp_stake,
        exchange_rate_stake_uluna,

        user_share,
        user_received_asset,

        tvl_uluna: exchange_rate_stake_uluna.checked_mul_uint(stake_balance)?,
    })
}

pub fn share(deps: Deps, env: Env, addr: Option<String>) -> StdResult<ShareResponse> {
    let state = State::default();
    let lp_token = state.lp_token.load(deps.storage)?;
    let stake_token = state.stake_token.load(deps.storage)?;
    let stake_extracted = state.stake_extracted.load(deps.storage)?;

    let total_lp = query_cw20_total_supply(&deps.querier, &lp_token)?;

    let stake_balance = query_cw20_balance(&deps.querier, &stake_token, &env.contract.address)?;

    let stake_available = stake_balance.checked_sub(stake_extracted)?;

    let share = if let Some(addr) = addr {
        query_cw20_balance(&deps.querier, &lp_token, &deps.api.addr_validate(&addr)?)?
    } else {
        Uint128::zero()
    };

    let received_asset = compute_withdraw_amount(total_lp, share, stake_available);

    Ok(ShareResponse {
        received_asset,
        share,
        total_lp,
    })
}
