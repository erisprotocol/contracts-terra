use cosmwasm_std::{
    entry_point, from_binary, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdResult,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;

use eris::helper::unwrap_reply;
use eris::hub::{CallbackMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, ReceiveMsg};

use crate::constants::{CONTRACT_DENOM, CONTRACT_NAME, CONTRACT_VERSION};
use crate::error::{ContractError, ContractResult};
use crate::helpers::parse_received_fund;
use crate::state::State;
use crate::{execute, gov, queries};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult {
    execute::instantiate(deps, env, msg)
}

#[entry_point]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> ContractResult {
    let api = deps.api;
    match msg {
        ExecuteMsg::Receive(cw20_msg) => receive(deps, env, info, cw20_msg),
        ExecuteMsg::Bond {
            receiver,
        } => execute::bond(
            deps,
            env,
            receiver.map(|s| api.addr_validate(&s)).transpose()?.unwrap_or(info.sender),
            parse_received_fund(&info.funds, CONTRACT_DENOM)?,
            false,
        ),
        ExecuteMsg::Donate {} => execute::bond(
            deps,
            env,
            info.sender,
            parse_received_fund(&info.funds, CONTRACT_DENOM)?,
            true,
        ),
        ExecuteMsg::WithdrawUnbonded {
            receiver,
        } => execute::withdraw_unbonded(
            deps,
            env,
            info.sender.clone(),
            receiver.map(|s| api.addr_validate(&s)).transpose()?.unwrap_or(info.sender),
        ),
        ExecuteMsg::AddValidator {
            validator,
        } => execute::add_validator(deps, info.sender, validator),
        ExecuteMsg::RemoveValidator {
            validator,
        } => execute::remove_validator(deps, env, info.sender, validator),
        ExecuteMsg::TransferOwnership {
            new_owner,
        } => execute::transfer_ownership(deps, info.sender, new_owner),
        ExecuteMsg::DropOwnershipProposal {} => execute::drop_ownership_proposal(deps, info.sender),
        ExecuteMsg::AcceptOwnership {} => execute::accept_ownership(deps, info.sender),
        ExecuteMsg::Harvest {} => execute::harvest(deps, env),
        ExecuteMsg::TuneDelegations {} => execute::tune_delegations(deps, env, info.sender),
        ExecuteMsg::Rebalance {
            min_redelegation,
        } => execute::rebalance(deps, env, info.sender, min_redelegation),
        ExecuteMsg::Reconcile {} => execute::reconcile(deps, env),
        ExecuteMsg::SubmitBatch {} => execute::submit_batch(deps, env),
        ExecuteMsg::Vote {
            proposal_id,
            vote,
        } => gov::vote(deps, env, info, proposal_id, vote),
        ExecuteMsg::VoteWeighted {
            proposal_id,
            votes,
        } => gov::vote_weighted(deps, env, info, proposal_id, votes),
        ExecuteMsg::Callback(callback_msg) => callback(deps, env, info, callback_msg),
        ExecuteMsg::UpdateConfig {
            protocol_fee_contract,
            protocol_reward_fee,
            delegation_strategy,
            allow_donations,
            vote_operator,
            epoch_period,
            unbond_period,
        } => execute::update_config(
            deps,
            info.sender,
            protocol_fee_contract,
            protocol_reward_fee,
            delegation_strategy,
            allow_donations,
            vote_operator,
            epoch_period,
            unbond_period,
        ),
    }
}

fn receive(deps: DepsMut, env: Env, info: MessageInfo, cw20_msg: Cw20ReceiveMsg) -> ContractResult {
    let api = deps.api;
    match from_binary(&cw20_msg.msg)? {
        ReceiveMsg::QueueUnbond {
            receiver,
        } => {
            let state = State::default();

            let stake_token = state.stake_token.load(deps.storage)?;
            if info.sender != stake_token {
                return Err(ContractError::ExpectingStakeToken(info.sender.into()));
            }

            execute::queue_unbond(
                deps,
                env,
                api.addr_validate(&receiver.unwrap_or(cw20_msg.sender))?,
                cw20_msg.amount,
            )
        },
    }
}

fn callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    callback_msg: CallbackMsg,
) -> ContractResult {
    if env.contract.address != info.sender {
        return Err(ContractError::CallbackOnlyCalledByContract {});
    }

    match callback_msg {
        CallbackMsg::Reinvest {} => execute::reinvest(deps, env),

        CallbackMsg::CheckReceivedCoin {
            snapshot,
        } => execute::callback_received_coin(deps, env, snapshot),
    }
}

#[entry_point]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> ContractResult {
    match reply.id {
        1 => execute::register_stake_token(deps, unwrap_reply(reply)?),
        id => Err(ContractError::InvalidReplyId(id)),
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&queries::config(deps)?),
        QueryMsg::State {} => to_binary(&queries::state(deps, env)?),
        QueryMsg::PendingBatch {} => to_binary(&queries::pending_batch(deps)?),
        QueryMsg::PreviousBatch(id) => to_binary(&queries::previous_batch(deps, id)?),
        QueryMsg::PreviousBatches {
            start_after,
            limit,
        } => to_binary(&queries::previous_batches(deps, start_after, limit)?),
        QueryMsg::UnbondRequestsByBatch {
            id,
            start_after,
            limit,
        } => to_binary(&queries::unbond_requests_by_batch(deps, id, start_after, limit)?),
        QueryMsg::UnbondRequestsByUser {
            user,
            start_after,
            limit,
        } => to_binary(&queries::unbond_requests_by_user(deps, user, start_after, limit)?),
        QueryMsg::UnbondRequestsByUserDetails {
            user,
            start_after,
            limit,
        } => to_binary(&queries::unbond_requests_by_user_details(
            deps,
            user,
            start_after,
            limit,
            env,
        )?),
        QueryMsg::WantedDelegations {} => to_binary(&queries::wanted_delegations(deps, env)?),
        QueryMsg::SimulateWantedDelegations {
            period,
        } => to_binary(&queries::simulate_wanted_delegations(deps, env, period)?),
    }
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> ContractResult {
    // let contract_version = get_contract_version(deps.storage)?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        // .add_attribute("previous_contract_name", &contract_version.contract)
        // .add_attribute("previous_contract_version", &contract_version.version)
        .add_attribute("new_contract_name", CONTRACT_NAME)
        .add_attribute("new_contract_version", CONTRACT_VERSION))
}
