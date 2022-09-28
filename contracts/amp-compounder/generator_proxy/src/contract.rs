use crate::bond::{
    callback_after_bond_changed, callback_after_bond_claimed, callback_claim_rewards,
    callback_deposit, callback_withdraw, execute_claim_rewards, execute_deposit, execute_withdraw,
    query_deposit, query_pending_token,
};
use crate::error::ContractError;
use crate::model::{
    CallbackMsg, Config, Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg, StakingState,
};
use crate::oper::{
    execute_controller_vote, execute_send_income, execute_update_config, execute_update_parameters,
    query_config, validate_percentage,
};
use crate::query::{
    query_pool_info, query_reward_info, query_staker_info, query_staking_state, query_user_info,
};
use crate::staking::{
    callback_after_staking_claimed, execute_claim_income, execute_relock, execute_request_unstake,
    execute_stake, execute_withdraw_unstaked,
};
use crate::state::{CONFIG, OWNERSHIP_PROPOSAL, STAKING_STATE};
use astroport::asset::addr_validate_to_lower;
use astroport::common::{claim_ownership, drop_ownership_proposal, propose_new_owner};
use astroport_governance::utils::get_period;
use cosmwasm_std::{
    entry_point, from_binary, to_binary, Binary, Decimal, Deps, DepsMut, Empty, Env, MessageInfo,
    Response, StdError, Uint128,
};
use cw20::Cw20ReceiveMsg;
use eris::adapters::generator::Generator;

/// ## Description
/// Creates a new contract with the specified parameters in the [`InstantiateMsg`].
/// Returns the [`Response`] with the specified attributes if the operation was successful, or a [`ContractError`] if the contract was not created.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    validate_percentage(msg.staker_rate, "staker_rate")?;
    validate_percentage(msg.boost_fee, "boost_fee")?;

    let config = Config {
        generator: Generator(addr_validate_to_lower(deps.api, &msg.generator)?),
        astro_gov: msg.astro_gov.check(deps.api)?,
        owner: addr_validate_to_lower(deps.api, &msg.owner)?,
        controller: addr_validate_to_lower(deps.api, &msg.controller)?,
        astro_token: addr_validate_to_lower(deps.api, &msg.astro_token)?,
        fee_collector: addr_validate_to_lower(deps.api, &msg.fee_collector)?,
        max_quota: msg.max_quota,
        staker_rate: msg.staker_rate,
        boost_fee: msg.boost_fee,
    };
    CONFIG.save(deps.storage, &config)?;

    let period = get_period(env.block.time.seconds())?;
    let state = StakingState {
        total_bond_share: Uint128::zero(),
        reward_index: Decimal::zero(),
        next_claim_period: period,
        total_unstaking_amount: Uint128::zero(),
        total_unstaked_amount: Uint128::zero(),
        unstaking_period: period,
    };
    STAKING_STATE.save(deps.storage, &state)?;

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
        ExecuteMsg::Receive(cw20_msg) => receive_cw20(deps, env, info, cw20_msg),
        ExecuteMsg::Callback(callback_msg) => handle_callback(deps, env, info, callback_msg),

        ExecuteMsg::UpdateConfig {
            controller,
            boost_fee,
        } => execute_update_config(deps, env, info, controller, boost_fee),
        ExecuteMsg::UpdateParameters {
            max_quota,
            staker_rate,
        } => execute_update_parameters(deps, env, info, max_quota, staker_rate),

        ExecuteMsg::ControllerVote {
            votes,
        } => execute_controller_vote(deps, env, info, votes),
        ExecuteMsg::SendIncome {} => execute_send_income(deps, env, info),

        ExecuteMsg::ClaimRewards {
            lp_tokens,
        } => execute_claim_rewards(deps, env, info, lp_tokens),
        ExecuteMsg::Withdraw {
            lp_token,
            amount,
        } => execute_withdraw(deps, env, info, lp_token, amount),

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

        ExecuteMsg::Relock {} => execute_relock(deps, env, info),
        ExecuteMsg::RequestUnstake {
            amount,
        } => execute_request_unstake(deps, env, info, amount),
        ExecuteMsg::WithdrawUnstaked {
            amount,
        } => execute_withdraw_unstaked(deps, env, info, amount),
        ExecuteMsg::ClaimIncome {} => execute_claim_income(deps, env, info),
    }
}

/// ## Description
/// Receives a message of type [`Cw20ReceiveMsg`] and processes it depending on the received template.
/// If the template is not found in the received message, then a [`ContractError`] is returned,
/// otherwise returns a [`Response`] with the specified attributes if the operation was successful
fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let staker_addr = addr_validate_to_lower(deps.api, &cw20_msg.sender)?;
    match from_binary(&cw20_msg.msg)? {
        Cw20HookMsg::Deposit {} => execute_deposit(deps, env, info, staker_addr, cw20_msg.amount),
        Cw20HookMsg::Stake {} => execute_stake(deps, env, info, staker_addr, cw20_msg.amount),
    }
}

/// # Description
/// Handle the callbacks describes in the [`CallbackMsg`]. Returns an [`ContractError`] on failure, otherwise returns the [`Response`]
fn handle_callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: CallbackMsg,
) -> Result<Response, ContractError> {
    // Callback functions can only be called by this contract itself
    if info.sender != env.contract.address {
        return Err(ContractError::CallbackUnauthorized {});
    }
    match msg {
        CallbackMsg::AfterBondClaimed {
            lp_token,
            prev_balances,
        } => callback_after_bond_claimed(deps, env, lp_token, prev_balances),
        CallbackMsg::Deposit {
            lp_token,
            staker_addr,
            amount,
        } => callback_deposit(deps, env, lp_token, staker_addr, amount),
        CallbackMsg::Withdraw {
            lp_token,
            staker_addr,
            amount,
        } => callback_withdraw(deps, env, lp_token, staker_addr, amount),
        CallbackMsg::AfterBondChanged {
            lp_token,
        } => callback_after_bond_changed(deps, env, lp_token),
        CallbackMsg::ClaimRewards {
            lp_token,
            staker_addr,
        } => callback_claim_rewards(deps, env, lp_token, staker_addr),
        CallbackMsg::AfterStakingClaimed {
            prev_balance,
        } => callback_after_staking_claimed(deps, env, prev_balance),
    }
}

/// ## Description
/// Exposes all the queries available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let result = match msg {
        QueryMsg::PendingToken {
            lp_token,
            user,
        } => to_binary(&query_pending_token(deps, env, lp_token, user)?),
        QueryMsg::Deposit {
            lp_token,
            user,
        } => to_binary(&query_deposit(deps, env, lp_token, user)?),
        QueryMsg::Config {} => to_binary(&query_config(deps, env)?),
        QueryMsg::PoolInfo {
            lp_token,
        } => to_binary(&query_pool_info(deps, env, lp_token)?),
        QueryMsg::UserInfo {
            lp_token,
            user,
        } => to_binary(&query_user_info(deps, env, lp_token, user)?),
        QueryMsg::RewardInfo {
            token,
        } => to_binary(&query_reward_info(deps, env, token)?),
        QueryMsg::StakingState {} => to_binary(&query_staking_state(deps, env)?),
        QueryMsg::StakerInfo {
            user,
        } => to_binary(&query_staker_info(deps, env, user)?),
    }?;
    Ok(result)
}

/// ## Description
/// Used for contract migration. Returns a default object of type [`Response`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: Empty) -> Result<Response, ContractError> {
    Ok(Response::default())
}
