use std::vec;

use astroport::asset::Asset;
use cosmwasm_std::{
    entry_point, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;

use eris::adapters::asset::AssetEx;
use eris::ampz::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

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
        } => crate::domain::execute::execute_id(deps, env, info, id),
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

    let mut msgs: Vec<CosmosMsg> = vec![];
    for asset in assets {
        asset.deposit_asset(&info, &env.contract.address, &mut msgs)?;
    }

    Ok(Response::new().add_attribute("action", "ampz/deposit").add_messages(msgs))
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&queries::config(deps)?),
        QueryMsg::State {} => to_binary(&queries::state(deps)?),
        QueryMsg::UserInfo {
            user,
        } => to_binary(&queries::user_info(deps, env, user)?),
        QueryMsg::Executions {
            limit,
            start_after,
        } => to_binary(&queries::executions(deps, start_after, limit)?),
        QueryMsg::Execution {
            id,
        } => to_binary(&queries::execution(deps, env, id)?),
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
