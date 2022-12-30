use std::vec;

use astroport::asset::Asset;
use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, Event, MessageInfo,
    Response, StdError, StdResult,
};
use cw2::set_contract_version;

use eris::adapters::asset::AssetEx;
use eris::ampz::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

use crate::config::update_config;
use crate::constants::{CONTRACT_NAME, CONTRACT_VERSION};
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
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Execute {
            id,
            user,
        } => crate::domain::execute::execute_id(deps, env, info, id, user),
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
        } => transfer_ownership(deps, info.sender, new_owner),
        ExecuteMsg::AcceptOwnership {} => accept_ownership(deps, info.sender),
        ExecuteMsg::UpdateConfig {
            ..
        } => update_config(deps, env, info, msg),

        ExecuteMsg::Deposit {
            assets,
        } => deposit(deps, env, info, assets),
    }
}

fn deposit(deps: DepsMut, env: Env, info: MessageInfo, assets: Vec<Asset>) -> StdResult<Response> {
    let state = State::default();
    if state.is_executing.load(deps.storage).is_err() {
        return Err(StdError::generic_err("no execution at the moment"));
    }

    let mut msgs: Vec<CosmosMsg> = vec![];
    for asset in assets {
        asset.deposit_asset(&info, &env.contract.address, &mut msgs)?;
    }

    Ok(Response::new().add_attribute("action", "ampz/deposit").add_messages(msgs))
}

pub fn transfer_ownership(deps: DepsMut, sender: Addr, new_owner: String) -> StdResult<Response> {
    let state = State::default();

    state.assert_owner(deps.storage, &sender)?;
    state.new_owner.save(deps.storage, &deps.api.addr_validate(&new_owner)?)?;

    Ok(Response::new().add_attribute("action", "ampz/transfer_ownership"))
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

    let event = Event::new("ampz/ownership_transferred")
        .add_attribute("new_owner", new_owner)
        .add_attribute("previous_owner", previous_owner);

    Ok(Response::new().add_event(event).add_attribute("action", "ampz/transfer_ownership"))
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
