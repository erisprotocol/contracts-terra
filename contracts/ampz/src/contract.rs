use std::collections::HashSet;
use std::vec;

use astroport::asset::Asset;
use cosmwasm_std::{
    attr, entry_point, to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order,
    Response, StdResult,
};
use cw2::set_contract_version;

use eris::adapters::asset::AssetEx;
use eris::ampz::{ClaimType, ExecuteMsg, Execution, InstantiateMsg, MigrateMsg, QueryMsg, Source};

use crate::constants::{CONTRACT_NAME, CONTRACT_VERSION};
use crate::error::{ContractError, ContractResult};
use crate::instantiate::exec_instantiate;
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
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> ContractResult {
    match msg {
        ExecuteMsg::Execute {
            id,
        } => crate::domain::execute::execute_id(deps, env, info, id.u128()),
        ExecuteMsg::AddExecution {
            execution,
            overwrite,
        } => crate::domain::crud::add_execution(deps, env, info, execution, overwrite),
        ExecuteMsg::RemoveExecutions {
            ids,
        } => crate::domain::crud::remove_executions(deps, env, info, ids),
        ExecuteMsg::Callback(callback_msg) => {
            crate::domain::callback::callback(deps, env, info, callback_msg)
        },
        ExecuteMsg::TransferOwnership {
            new_owner,
        } => crate::domain::ownership::transfer_ownership(deps, info.sender, new_owner),
        ExecuteMsg::DropOwnershipProposal {} => {
            crate::domain::ownership::drop_ownership_proposal(deps, info.sender)
        },
        ExecuteMsg::AcceptOwnership {} => {
            crate::domain::ownership::accept_ownership(deps, info.sender)
        },
        ExecuteMsg::UpdateConfig {
            ..
        } => crate::domain::config::update_config(deps, env, info, msg),

        ExecuteMsg::Deposit {
            assets,
        } => deposit(deps, env, info, assets),
    }
}

fn deposit(deps: DepsMut, env: Env, info: MessageInfo, assets: Vec<Asset>) -> ContractResult {
    let state = State::default();
    if state.is_executing.load(deps.storage).is_err() {
        return Err(ContractError::IsNotExecuting {});
    }

    let mut uniq = HashSet::new();
    if !assets.clone().into_iter().all(|a| uniq.insert(a.info.to_string())) {
        return Err(ContractError::DuplicatedAsset {});
    }

    let mut msgs: Vec<CosmosMsg> = vec![];
    for asset in assets {
        asset.deposit_asset(&info, &env.contract.address, &mut msgs)?;
    }

    Ok(Response::new().add_attribute("action", "ampz/deposit").add_messages(msgs))
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&queries::config(deps)?),
        QueryMsg::State {} => to_json_binary(&queries::state(deps)?),
        QueryMsg::UserInfo {
            user,
        } => to_json_binary(&queries::user_info(deps, env, user)?),
        QueryMsg::Executions {
            limit,
            start_after,
        } => to_json_binary(&queries::executions(deps, start_after, limit)?),
        QueryMsg::ExecutionsSchedule {
            limit,
            start_after,
        } => to_json_binary(&queries::executions_schedule(deps, start_after, limit)?),
        QueryMsg::Execution {
            id,
        } => to_json_binary(&queries::execution(deps, env, id)?),
    }
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // let state = State::default();
    // let mut attrs = vec![];
    // let mut removed_count = 0u32;

    // // Iterate through all executions and remove alliance/whitewhale rewards
    // let all_executions: Vec<(u128, Execution)> = state
    //     .executions
    //     .range(deps.storage, None, None, Order::Ascending)
    //     .collect::<StdResult<Vec<_>>>()?;

    // for (id, execution) in all_executions {
    //     let should_remove = match &execution.source {
    //         // Remove WhiteWhaleRewards source
    //         Source::WhiteWhaleRewards {
    //             ..
    //         } => true,
    //         // Remove ClaimContract with WhiteWhaleRewards claim type
    //         Source::ClaimContract {
    //             claim_type: ClaimType::WhiteWhaleRewards,
    //         } => true,
    //         // Remove ClaimContract with AllianceRewards claim type
    //         Source::ClaimContract {
    //             claim_type: ClaimType::AllianceRewards,
    //         } => true,
    //         // Keep all other sources
    //         _ => false,
    //     };

    //     if should_remove {
    //         state.executions.remove(deps.storage, id)?;
    //         state.last_execution.remove(deps.storage, id);

    //         let source = execution.source.try_get_uniq_key();
    //         if let Some(source) = source {
    //             state.execution_user_source.remove(deps.storage, (execution.user, source));
    //         }

    //         attrs.push(attr("removed_id", id.to_string()));
    //         removed_count += 1;
    //     }
    // }

    Ok(Response::new()
        .add_attribute("new_contract_name", CONTRACT_NAME)
        .add_attribute("new_contract_version", CONTRACT_VERSION))
}
