use std::vec;

use astroport::asset::{native_asset_info, Asset};
use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, Event, MessageInfo,
    Response, StdError, StdResult,
};
use cw2::set_contract_version;

use eris::adapters::farm::Farm;
use eris::ampz::{
    CallbackMsg, CallbackWrapper, ExecuteMsg, Execution, InstantiateMsg, MigrateMsg, QueryMsg,
};
use eris::hub::ExecuteMsg as HubExecuteMsg;
use itertools::Itertools;
use protobuf::SpecialFields;

use crate::config::update_config;
use crate::constants::{CONTRACT_DENOM, CONTRACT_NAME, CONTRACT_VERSION};
use crate::helpers::{funds_or_allowance, query_all_delegations};
use crate::instantiate::exec_instantiate;
use crate::protos::authz::MsgExec;
use crate::protos::msgex::{CosmosMsgEx, MsgExecuteContractEx};
use crate::protos::proto::{Coin, MsgExecuteContract, MsgWithdrawDelegatorReward};
use crate::queries;
use crate::state::State;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    exec_instantiate(deps, env, msg)
}

#[entry_point]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Execute {
            id,
        } => execute_id(deps, env, info, id),
        ExecuteMsg::AddExecution {
            execution,
            overwrite,
        } => add_execution(deps, env, info, execution, overwrite),
        ExecuteMsg::RemoveExecutions {
            ids,
        } => remove_executions(deps, env, info, ids),
        ExecuteMsg::Callback(callback_msg) => callback(deps, env, info, callback_msg),
        ExecuteMsg::TransferOwnership {
            new_owner,
        } => transfer_ownership(deps, info.sender, new_owner),
        ExecuteMsg::AcceptOwnership {} => accept_ownership(deps, info.sender),

        ExecuteMsg::UpdateConfig {
            ..
        } => update_config(deps, env, info, msg),
        // ExecuteMsg::AddToTipJar {
        //     recipient,
        // } => {
        //     let recipient = if let Some(recipient) = recipient {
        //         deps.api.addr_validate(&recipient)
        //     } else {
        //         Ok(info.sender.clone())
        //     }?;

        //     add_to_tip_jar(deps, info, recipient)
        // },

        // ExecuteMsg::WithdrawTipJar {
        //     amount,
        // } => withdraw_tip_jar(deps, info, amount),
    }
}

fn remove_executions(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    ids: Option<Vec<u128>>,
) -> StdResult<Response> {
    let state = State::default();

    if let Some(ids) = ids {
        // if ids specified remove all ids
        for id in ids {
            let execution = state.get_by_id(deps.storage, id)?;

            if execution.user != info.sender {
                return Err(StdError::generic_err("can only be removed by creator"));
            }

            state.executions.remove(deps.storage, id)?;
            state
                .execution_user_source
                .remove(deps.storage, (execution.user, execution.source.into()));
        }
    } else {
        // if nothing specified remove all from user
        let executions = state.get_by_user(deps.storage, info.sender.to_string())?;
        for execution in executions {
            state.executions.remove(deps.storage, execution.0)?;
            state
                .execution_user_source
                .remove(deps.storage, (execution.1.user, execution.1.source.into()));
        }
    }

    Ok(Response::new().add_attribute("action", "remove_executions"))
}

fn add_execution(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    execution: Execution,
    overwrite: bool,
) -> StdResult<Response> {
    if execution.user != info.sender {
        return Err(StdError::generic_err("can only be added by same user"));
    }

    let state = State::default();
    let source: String = execution.source.clone().into();

    if overwrite {
        let result = state
            .execution_user_source
            .load(deps.storage, (execution.user.to_string(), source.clone()));

        if let Ok(id) = result {
            state.executions.remove(deps.storage, id)?;
            state
                .execution_user_source
                .remove(deps.storage, (execution.user.clone(), source.clone()));
        }
    } else if state
        .execution_user_source
        .has(deps.storage, (execution.user.to_string(), source.clone()))
    {
        return Err(StdError::generic_err("source already defined for the user"));
    }

    match execution.destination.clone() {
        CallbackMsg::DepositAmplifier {} => (),
        CallbackMsg::DepositFarm {
            farm,
        } => {
            let allowed_farms = state.farms.load(deps.storage)?;
            let farm = Farm(deps.api.addr_validate(&farm)?);
            if !allowed_farms.contains(&farm) {
                return Err(StdError::generic_err(format!("farm {} does not exist", farm.0)));
            }
        },
        // CallbackMsg::DepositFarms {
        //     farms,
        // } => {
        //     let allowed_farms = state.farms.load(deps.storage)?;
        //     let total: BasicPoints = 0u16.try_into()?;
        //     for farm in farms {
        //         total.checked_add(farm.1)?;
        //         let farm = Farm(deps.api.addr_validate(&farm.0)?);
        //         if !allowed_farms.contains(&farm) {
        //             return Err(StdError::generic_err(format!("farm {} does not exist", farm.0)));
        //         }
        //     }

        //     if !total.is_max() {
        //         return Err(StdError::generic_err("total must be 100%"));
        //     }
        // },
        _ => return Err(StdError::generic_err("callback is not allowed")),
    }

    let id = state.id.load(deps.storage)?;
    state.executions.save(deps.storage, id, &execution)?;
    state.execution_user_source.save(deps.storage, (execution.user, source), &id)?;

    state.id.save(deps.storage, &(id + 1))?;

    Ok(Response::new().add_attribute("action", "add_execution").add_attribute("id", id.to_string()))
}

fn execute_id(deps: DepsMut, env: Env, info: MessageInfo, id: u128) -> StdResult<Response> {
    let state = State::default();

    state.assert_controller_owner(deps.storage, &info.sender)?;

    let execution = state.get_by_id(deps.storage, id)?;
    let user = deps.api.addr_validate(&execution.user)?;
    let source: String = execution.source.clone().into();

    let last_execution = state.last_execution.load(deps.storage, id)?;
    let next_execution = last_execution
        .checked_add(execution.schedule.interval_s)
        .ok_or_else(|| StdError::generic_err("cannot add interval_s"))?;

    if next_execution > env.block.time.seconds() {
        return Err(StdError::generic_err(format!(
            "next execution in the future: {}",
            next_execution
        )));
    }

    state.last_execution.save(deps.storage, id, &env.block.time.seconds())?;

    let mut msgs: Vec<CosmosMsg> = vec![];

    match execution.source {
        eris::ampz::Source::Claim => {
            state.set_user_balance_snapshot(
                &deps.querier,
                deps.storage,
                &user,
                vec![native_asset_info(CONTRACT_DENOM.to_string())],
            )?;
            let delegations = query_all_delegations(&deps.querier, &user)?;

            let mut exec = MsgExec::new();
            exec.grantee = env.contract.address.clone().into();
            exec.msgs = vec![];

            for delegation in delegations {
                let mut msg = MsgWithdrawDelegatorReward::new();
                msg.delegator_address = user.to_string();
                msg.validator_address = delegation.validator.clone();
                exec.msgs.push(msg.to_any()?)
            }

            msgs.push(exec.to_cosmos_msg());
        },
        eris::ampz::Source::AstroRewards {
            lps,
        } => {
            let astroport = state.astroport.load(deps.storage)?;

            state.set_user_balance_snapshot(&deps.querier, deps.storage, &user, astroport.coins)?;

            let msg = astroport.generator.claim_rewards_msg(lps)?.to_authz_msg(user, &env)?;

            msgs.push(msg);

            if let CallbackMsg::DepositAmplifier {} = execution.destination {
                msgs.push(
                    CallbackMsg::MultiSwap {
                        into: native_asset_info(CONTRACT_DENOM.to_string()),
                    }
                    .into_cosmos_msg(&env.contract.address, id)?,
                )
            }
        },
    }

    msgs.push(execution.destination.into_cosmos_msg(&env.contract.address, id)?);

    Ok(Response::new()
        .add_attribute("action", "execute_id")
        .add_attribute("id", id.to_string())
        .add_attribute("source", source)
        .add_messages(msgs))
}

fn callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    callback_wrapper: CallbackWrapper,
) -> StdResult<Response> {
    if env.contract.address != info.sender {
        return Err(StdError::generic_err("callbacks can only be invoked by the contract itself"));
    }

    let state = State::default();
    let execution = state.get_by_id(deps.storage, callback_wrapper.id)?;
    let user = deps.api.addr_validate(&execution.user)?;

    let mut msgs: Vec<CosmosMsg> = vec![];

    match callback_wrapper.message {
        CallbackMsg::DepositAmplifier {} => {
            let balances =
                state.get_user_balance_diff_and_clear(&deps.querier, deps.storage, &user)?;
            let hub = state.hub.load(deps.storage)?;
            let main_token = native_asset_info(CONTRACT_DENOM.to_string());
            let amount = balances
                .iter()
                .find(|a| a.info == main_token)
                .ok_or_else(|| StdError::generic_err("main token not found"))?;

            let execute_contract = MsgExecuteContract {
                sender: user.to_string(),
                contract: hub.0.to_string(),
                msg: to_binary(&HubExecuteMsg::Bond {
                    receiver: None,
                })?
                .to_vec(),
                funds: vec![Coin {
                    amount: amount.amount.to_string(),
                    denom: CONTRACT_DENOM.to_string(),
                    special_fields: SpecialFields::default(),
                }],
                special_fields: SpecialFields::default(),
            };

            msgs.push(execute_contract.to_cosmos_msg(&env)?);
        },
        CallbackMsg::DepositFarm {
            farm,
        } => {
            let balances =
                state.get_user_balance_diff_and_clear(&deps.querier, deps.storage, &user)?;
            deposit_in_farm(&deps, farm, &env, &user, balances, &mut msgs)?;
        },

        CallbackMsg::MultiSwap {
            into,
        } => {
            let balances = state.get_user_balance_diff(&deps.querier, deps.storage, &user)?;
            let zapper = state.zapper.load(deps.storage)?;

            let to_swap = balances.into_iter().filter(|asset| asset.info != into).collect_vec();

            let (funds, mut allowances) = funds_or_allowance(&env, &user, &zapper.0, &to_swap)?;

            msgs.append(&mut allowances);
            msgs.push(
                zapper
                    .multi_swap_msg(to_swap, into, funds, Some(user.to_string()))?
                    .to_authz_msg(user, &env)?,
            )
        },
    }

    Ok(Response::new().add_attribute("action", "callback").add_messages(msgs))
}

fn deposit_in_farm(
    deps: &DepsMut,
    farm: String,
    env: &Env,
    user: &Addr,
    balances: Vec<Asset>,
    msgs: &mut Vec<CosmosMsg>,
) -> Result<(), StdError> {
    let farm = deps.api.addr_validate(&farm)?;
    let (funds, mut allowances) = funds_or_allowance(env, user, &farm, &balances)?;
    msgs.append(&mut allowances);
    msgs.push(Farm(farm).bond_assets(balances, funds)?.to_authz_msg(user, env)?);
    Ok(())
}

pub fn transfer_ownership(deps: DepsMut, sender: Addr, new_owner: String) -> StdResult<Response> {
    let state = State::default();

    state.assert_owner(deps.storage, &sender)?;
    state.new_owner.save(deps.storage, &deps.api.addr_validate(&new_owner)?)?;

    Ok(Response::new().add_attribute("action", "erishub/transfer_ownership"))
}

pub fn accept_ownership(deps: DepsMut, sender: Addr) -> StdResult<Response> {
    let state = State::default();

    let previous_owner = state.owner.load(deps.storage)?;
    let new_owner = state.new_owner.load(deps.storage)?;

    if sender != new_owner {
        return Err(StdError::generic_err("unauthorized: sender is not new owner"));
    }

    state.owner.save(deps.storage, &sender)?;
    state.new_owner.remove(deps.storage);

    let event = Event::new("erishub/ownership_transferred")
        .add_attribute("new_owner", new_owner)
        .add_attribute("previous_owner", previous_owner);

    Ok(Response::new().add_event(event).add_attribute("action", "erishub/transfer_ownership"))
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&queries::config(deps)?),
        QueryMsg::State {} => to_binary(&queries::state(deps)?),
        QueryMsg::UserInfo {
            user,
        } => to_binary(&queries::user_info(deps, user)?),
        QueryMsg::Executions {
            limit,
            start_after,
        } => to_binary(&queries::executions(deps, start_after, limit)?),
    }
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    // let contract_version = get_contract_version(deps.storage)?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        // .add_attribute("previous_contract_name", &contract_version.contract)
        // .add_attribute("previous_contract_version", &contract_version.version)
        .add_attribute("new_contract_name", CONTRACT_NAME)
        .add_attribute("new_contract_version", CONTRACT_VERSION))
}
