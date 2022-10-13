use crate::error::ContractError;
use crate::model::{CallbackMsg, Config, ExecuteMsg, RewardInfo, StakerInfo, StakingState};
use crate::state::{CONFIG, REWARD_INFO, STAKER_INFO, STAKING_STATE};
use astroport::asset::token_asset;
use astroport::querier::query_token_balance;
use astroport_governance::utils::{get_period, WEEK};
use cosmwasm_std::{
    Addr, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, QuerierWrapper, Response, StdResult,
    Uint128,
};
use eris::adapters::asset::AssetEx;
use std::cmp;

pub fn execute_stake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    staker_addr: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // deposited token must be xastro
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.astro_gov.xastro_token {
        return Err(ContractError::Unauthorized {});
    }

    // check quota
    let lock = config.astro_gov.query_lock(&deps.querier, env.contract.address.clone())?;
    if lock.amount + amount > config.max_quota {
        return Err(ContractError::ExceedQuota(config.max_quota.saturating_sub(lock.amount)));
    }

    // stake to voting escrow
    let mut messages: Vec<CosmosMsg> = vec![];
    if lock.amount.is_zero() {
        let lock_msg = config.astro_gov.create_lock_msg(amount, WEEK)?;
        messages.push(lock_msg);
    } else {
        if lock.end <= get_period(env.block.time.seconds())? {
            let relock_msg = ExecuteMsg::Relock {}.to_cosmos_msg(&env.contract.address)?;
            messages.push(relock_msg);
        }
        let lock_msg = config.astro_gov.extend_lock_amount_msg(amount)?;
        messages.push(lock_msg);
    }

    let mut astro_reward = REWARD_INFO.load(deps.storage, &config.astro_token)?;
    let mut state = STAKING_STATE.load(deps.storage)?;
    let mut staker_info = STAKER_INFO
        .may_load(deps.storage, &staker_addr)?
        .unwrap_or_else(|| StakerInfo::create(&state));

    reconcile_staker_income(&mut astro_reward, &mut state)?;
    reconcile_to_staker_info(&state, &mut staker_info)?;

    let share = state.calc_bond_share(lock.amount, amount, false);
    staker_info.bond_share += share;
    state.total_bond_share += share;

    STAKER_INFO.save(deps.storage, &staker_addr, &staker_info)?;
    STAKING_STATE.save(deps.storage, &state)?;
    REWARD_INFO.save(deps.storage, &config.astro_token, &astro_reward)?;

    Ok(Response::new().add_messages(messages).add_attribute("add_share", share))
}

pub fn reconcile_staker_income(
    astro_reward: &mut RewardInfo,
    state: &mut StakingState,
) -> StdResult<()> {
    if !state.total_bond_share.is_zero() {
        let income_per_share =
            Decimal::from_ratio(astro_reward.staker_income, state.total_bond_share);
        astro_reward.staker_income = Uint128::zero();
        state.reward_index += income_per_share;
    }

    Ok(())
}

pub fn reconcile_to_staker_info(
    state: &StakingState,
    staker_info: &mut StakerInfo,
) -> StdResult<()> {
    staker_info.pending_reward +=
        (state.reward_index - staker_info.reward_index) * staker_info.bond_share;
    staker_info.reward_index = state.reward_index;

    Ok(())
}

pub fn execute_relock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // only self & controller can relock
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.controller && info.sender != env.contract.address {
        return Err(ContractError::Unauthorized {});
    }

    // relock
    let mut astro_reward = REWARD_INFO.load(deps.storage, &config.astro_token)?;
    let mut state = STAKING_STATE.load(deps.storage)?;
    let lock = config.astro_gov.query_lock(&deps.querier, env.contract.address.clone())?;
    let lock_amount = lock.amount.checked_sub(state.total_unstaking_amount)?;
    state.total_unstaked_amount += state.total_unstaking_amount;
    state.total_unstaking_amount = Uint128::zero();
    state.unstaking_period = get_period(env.block.time.seconds())?;

    // reconcile rewards
    let prev_balance = reconcile_staking_claim_by_others(
        &deps.querier,
        &env,
        &config,
        &mut astro_reward,
        &mut state,
    )?;

    // save
    STAKING_STATE.save(deps.storage, &state)?;
    REWARD_INFO.save(deps.storage, &config.astro_token, &astro_reward)?;

    Ok(Response::new()
        .add_message(config.astro_gov.claim_msg()?)
        .add_message(
            CallbackMsg::AfterStakingClaimed {
                prev_balance,
            }
            .to_cosmos_msg(&env.contract.address)?,
        )
        .add_message(config.astro_gov.withdraw_msg()?)
        .add_message(config.astro_gov.create_lock_msg(lock_amount, WEEK)?))
}

fn reconcile_staking_claim_by_others(
    querier: &QuerierWrapper,
    env: &Env,
    config: &Config,
    astro_reward: &mut RewardInfo,
    state: &mut StakingState,
) -> StdResult<Uint128> {
    // calculate claim
    let current_period =
        config.astro_gov.query_last_claim_period(querier, env.contract.address.clone())?;
    let target_add_astro_amount = config.astro_gov.calc_claim_amount(
        querier,
        env.contract.address.clone(),
        state.next_claim_period,
        current_period,
    )?;
    state.next_claim_period = current_period;

    // update amount
    let astro_amount = query_token_balance(querier, &config.astro_token, &env.contract.address)?;
    let add_astro_amount = astro_amount.saturating_sub(astro_reward.reconciled_amount);
    let net_astro_amount = cmp::min(add_astro_amount, target_add_astro_amount);
    astro_reward.staker_income += net_astro_amount;
    astro_reward.reconciled_amount += net_astro_amount;
    reconcile_staker_income(astro_reward, state)?;

    Ok(astro_amount)
}

pub fn callback_after_staking_claimed(
    deps: DepsMut,
    env: Env,
    prev_balance: Uint128,
) -> Result<Response, ContractError> {
    // load data
    let config = CONFIG.load(deps.storage)?;
    let mut state = STAKING_STATE.load(deps.storage)?;
    let mut astro_reward = REWARD_INFO.load(deps.storage, &config.astro_token)?;

    // calculate claim
    let current_period =
        config.astro_gov.query_last_claim_period(&deps.querier, env.contract.address.clone())?;
    state.next_claim_period = current_period;

    // update amount
    let balance = query_token_balance(&deps.querier, &config.astro_token, env.contract.address)?;
    let net_astro_amount = balance.checked_sub(prev_balance)?;
    astro_reward.staker_income += net_astro_amount;
    astro_reward.reconciled_amount += net_astro_amount;
    reconcile_staker_income(&mut astro_reward, &mut state)?;

    // save
    REWARD_INFO.save(deps.storage, &config.astro_token, &astro_reward)?;
    STAKING_STATE.save(deps.storage, &state)?;

    Ok(Response::default())
}

pub fn execute_request_unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // load
    let config = CONFIG.load(deps.storage)?;
    let mut astro_reward = REWARD_INFO.load(deps.storage, &config.astro_token)?;
    let mut state = STAKING_STATE.load(deps.storage)?;
    let mut staker_info = STAKER_INFO.load(deps.storage, &info.sender)?;

    // update
    reconcile_staker_income(&mut astro_reward, &mut state)?;
    reconcile_to_staker_info(&state, &mut staker_info)?;
    staker_info.update_staking(&state);
    let lock = config.astro_gov.query_lock(&deps.querier, env.contract.address)?;
    let share = state.calc_bond_share(lock.amount, amount, true);
    staker_info.bond_share = staker_info.bond_share.checked_sub(share)?;
    staker_info.unstaking_amount += amount;
    state.total_bond_share = state.total_bond_share.checked_sub(share)?;
    state.total_unstaking_amount += amount;

    // save
    STAKER_INFO.save(deps.storage, &info.sender, &staker_info)?;
    STAKING_STATE.save(deps.storage, &state)?;
    REWARD_INFO.save(deps.storage, &config.astro_token, &astro_reward)?;

    Ok(Response::new().add_attribute("deduct_share", share))
}

pub fn execute_withdraw_unstaked(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    // load
    let config = CONFIG.load(deps.storage)?;
    let mut state = STAKING_STATE.load(deps.storage)?;
    let mut staker_info = STAKER_INFO.load(deps.storage, &info.sender)?;

    // update
    staker_info.update_staking(&state);
    let amount = amount.unwrap_or(staker_info.unstaked_amount);
    staker_info.unstaked_amount = staker_info.unstaked_amount.checked_sub(amount)?;
    state.total_unstaked_amount = state.total_unstaked_amount.checked_sub(amount)?;

    // save
    STAKER_INFO.save(deps.storage, &info.sender, &staker_info)?;
    STAKING_STATE.save(deps.storage, &state)?;

    // message
    let transfer_msg =
        token_asset(config.astro_gov.xastro_token, amount).transfer_msg(&info.sender)?;
    Ok(Response::new().add_message(transfer_msg))
}

pub fn execute_claim_income(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // load
    let config = CONFIG.load(deps.storage)?;
    let mut astro_reward = REWARD_INFO.load(deps.storage, &config.astro_token)?;
    let mut state = STAKING_STATE.load(deps.storage)?;
    let mut staker_info = STAKER_INFO.load(deps.storage, &info.sender)?;

    // update
    reconcile_staker_income(&mut astro_reward, &mut state)?;
    reconcile_to_staker_info(&state, &mut staker_info)?;
    let amount = staker_info.pending_reward;
    staker_info.pending_reward = Uint128::zero();
    astro_reward.reconciled_amount = astro_reward.reconciled_amount.checked_sub(amount)?;

    // save
    STAKER_INFO.save(deps.storage, &info.sender, &staker_info)?;
    STAKING_STATE.save(deps.storage, &state)?;
    REWARD_INFO.save(deps.storage, &config.astro_token, &astro_reward)?;

    let transfer_msg = token_asset(config.astro_token, amount).transfer_msg(&info.sender)?;
    Ok(Response::new().add_message(transfer_msg))
}
