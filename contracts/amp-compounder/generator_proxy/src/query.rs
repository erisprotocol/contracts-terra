use cosmwasm_std::{Deps, Env, StdResult};
use astroport::asset::addr_validate_to_lower;
use crate::bond::reconcile_to_user_info;
use crate::model::{PoolInfo, RewardInfo, StakerInfo, StakerInfoResponse, StakingState, UserInfo, UserInfoResponse};
use crate::staking::{reconcile_staker_income, reconcile_to_staker_info};
use crate::state::{CONFIG, POOL_INFO, REWARD_INFO, STAKER_INFO, STAKING_STATE, USER_INFO};

pub fn query_pool_info(
    deps: Deps,
    _env: Env,
    lp_token: String,
) -> StdResult<PoolInfo> {
    let lp_token = addr_validate_to_lower(deps.api, &lp_token)?;
    POOL_INFO.load(deps.storage, &lp_token)
}

pub fn query_user_info(
    deps: Deps,
    env: Env,
    lp_token: String,
    user: String,
) -> StdResult<UserInfoResponse> {
    let lp_token = addr_validate_to_lower(deps.api, &lp_token)?;
    let user = addr_validate_to_lower(deps.api, &user)?;
    let pool_info = POOL_INFO.load(deps.storage, &lp_token)?;
    let mut user_info = USER_INFO.may_load(deps.storage, (&lp_token, &user))?
        .unwrap_or_else(|| UserInfo::create(&pool_info));
    reconcile_to_user_info(&pool_info, &mut user_info)?;

    let config = CONFIG.load(deps.storage)?;
    let total_bond_amount = config.generator.query_deposit(&deps.querier, &lp_token, &env.contract.address)?;
    Ok(user_info.to_response(&pool_info, total_bond_amount))
}

pub fn query_reward_info(
    deps: Deps,
    _env: Env,
    token: String,
) -> StdResult<RewardInfo> {
    let token = addr_validate_to_lower(deps.api, &token)?;
    REWARD_INFO.load(deps.storage, &token)
}

pub fn query_staking_state(
    deps: Deps,
    _env: Env,
) -> StdResult<StakingState> {
    STAKING_STATE.load(deps.storage)
}

pub fn query_staker_info(
    deps: Deps,
    env: Env,
    user: String,
) -> StdResult<StakerInfoResponse> {
    let user = addr_validate_to_lower(deps.api, &user)?;
    let config = CONFIG.load(deps.storage)?;
    let mut astro_reward = REWARD_INFO.load(deps.storage, &config.astro_token)?;
    let mut state = STAKING_STATE.load(deps.storage)?;
    let mut staker_info = STAKER_INFO.may_load(deps.storage, &user)?
        .unwrap_or_else(|| StakerInfo::create(&state));
    reconcile_staker_income(&mut astro_reward, &mut state)?;
    reconcile_to_staker_info(&state, &mut staker_info)?;
    staker_info.update_staking(&state);

    let lock = config.astro_gov.query_lock(&deps.querier, env.contract.address)?;
    Ok(staker_info.to_response(&state, lock.amount))
}
