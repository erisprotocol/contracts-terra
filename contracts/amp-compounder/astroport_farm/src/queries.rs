use astroport::asset::addr_validate_to_lower;
use cosmwasm_std::{Decimal, Deps, Env, StdResult};
use eris::astroport_farm::{ConfigResponse, StateResponse, UserInfoResponse};

use crate::state::{CONFIG, STATE};

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
    })
}

/// ## Description
/// Returns contract state
pub fn query_state(deps: Deps, env: Env) -> StdResult<StateResponse> {
    let state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;

    let lp_token = config.lp_token;
    let total_lp =
        config.staking_contract.query_deposit(&deps.querier, &lp_token, &env.contract.address)?;

    let total_amp_lp = state.amp_lp_token.query_supply(&deps.querier)?;
    let total_share = state.total_bond_share;

    Ok(StateResponse {
        total_lp,
        total_amp_lp,
        total_share,
        exchange_rate: Decimal::from_ratio(total_lp, total_share),
    })
}

/// ## Description
/// Returns reward info for the staker.
pub fn query_user_info(deps: Deps, env: Env, staker_addr: String) -> StdResult<UserInfoResponse> {
    let staker_addr_validated = addr_validate_to_lower(deps.api, &staker_addr)?;

    let state = STATE.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;

    let staking_token = config.lp_token;

    let lp_balance = config.staking_contract.query_deposit(
        &deps.querier,
        &staking_token,
        &env.contract.address,
    )?;

    let bond_share = state.amp_lp_token.query_amount(&deps.querier, staker_addr_validated)?;
    let bond_amount = state.calc_bond_amount(lp_balance, bond_share);

    Ok(UserInfoResponse {
        lp_balance,
        total_share: state.total_bond_share,
        bond_amount,
        bond_share,
    })
}
