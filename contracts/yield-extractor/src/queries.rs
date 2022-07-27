use cosmwasm_std::{Decimal, Deps, Env, StdResult};

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

pub fn state(deps: Deps, env: Env) -> StdResult<StateResponse> {
    let state = State::default();

    let lp_token = state.lp_token.load(deps.storage)?;
    let total_lp = query_cw20_total_supply(&deps.querier, &lp_token)?;

    let extract_config = state.extract_config.load(deps.storage)?;
    let stake_token = state.stake_token.load(deps.storage)?;
    let stake_extracted = state.stake_extracted.load(deps.storage)?;
    let stake_harvested = state.stake_harvested.load(deps.storage)?;
    // let _last_exchange_rate = state.last_exchange_rate.load(deps.storage)?;

    let current_exchange_rate =
        query_exchange_rate(&deps.querier, extract_config.interface, &extract_config.hub_contract)?;

    let stake_in_contract = query_cw20_balance(&deps.querier, &stake_token, &env.contract.address)?;

    let stake_available = stake_in_contract.checked_sub(stake_extracted)?;

    let exchange_rate_lp_lsd = if total_lp.is_zero() {
        Decimal::zero()
    } else {
        Decimal::from_ratio(stake_available, total_lp)
    };

    Ok(StateResponse {
        total_lp,
        total_lsd: stake_in_contract,
        harvestable: stake_extracted,
        total_harvest: stake_harvested,
        exchange_rate_lp_lsd,
        exchange_rate_lsd_uluna: current_exchange_rate,

        tvl_uluna: current_exchange_rate.checked_mul_uint(stake_in_contract)?,
    })
}

pub fn share(deps: Deps, env: Env, addr: String) -> StdResult<ShareResponse> {
    let state = State::default();
    let lp_token = state.lp_token.load(deps.storage)?;
    let stake_token = state.stake_token.load(deps.storage)?;
    let stake_extracted = state.stake_extracted.load(deps.storage)?;

    let total_lp = query_cw20_total_supply(&deps.querier, &lp_token)?;

    let stake_balance = query_cw20_balance(&deps.querier, &stake_token, &env.contract.address)?;

    let stake_available = stake_balance.checked_sub(stake_extracted)?;

    let share = query_cw20_balance(&deps.querier, &lp_token, &deps.api.addr_validate(&addr)?)?;

    let received_asset = compute_withdraw_amount(total_lp, share, stake_available);

    Ok(ShareResponse {
        received_asset,
        share,
        total_lp,
    })
}
