use cosmwasm_std::{
    entry_point, from_binary, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdError, StdResult,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::Cw20ReceiveMsg;

use eris_staking::yieldextractor::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, ReceiveMsg};

use crate::constants::{CONTRACT_NAME, CONTRACT_VERSION};
use crate::helpers::unwrap_reply;
use crate::state::State;
use crate::{execute, queries};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    execute::instantiate(deps, env, msg)
}

#[entry_point]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Receive(cw20_msg) => receive(deps, env, info, cw20_msg),
        ExecuteMsg::TransferOwnership {
            new_owner,
        } => execute::transfer_ownership(deps, info.sender, new_owner),
        ExecuteMsg::AcceptOwnership {} => execute::accept_ownership(deps, info.sender),
        ExecuteMsg::Harvest {} => execute::harvest(deps, env, info.sender),
        ExecuteMsg::UpdateConfig {
            yield_extract_addr,
        } => execute::update_config(deps, info.sender, yield_extract_addr),
    }
}

fn receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> StdResult<Response> {
    let api = deps.api;
    match from_binary(&cw20_msg.msg)? {
        ReceiveMsg::Withdraw {} => {
            // receiving LP Token
            let state = State::default();

            let lp_token = state.lp_token.load(deps.storage)?;
            if info.sender != lp_token {
                return Err(StdError::generic_err(format!(
                    "expecting LP token, received {}",
                    info.sender
                )));
            }

            execute::withdraw(deps, env, api.addr_validate(&cw20_msg.sender)?, cw20_msg.amount)
        },
        ReceiveMsg::Deposit {} => {
            // receiving ampLUNA
            let state = State::default();

            let stake_token = state.stake_token.load(deps.storage)?;
            if info.sender != stake_token {
                return Err(StdError::generic_err(format!(
                    "expecting Stake token, received {}",
                    info.sender
                )));
            }

            execute::deposit(deps, env, api.addr_validate(&cw20_msg.sender)?, cw20_msg.amount)
        },
    }
}

#[entry_point]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> StdResult<Response> {
    match reply.id {
        1 => execute::register_lp_token(deps, unwrap_reply(reply)?),
        id => Err(StdError::generic_err(format!("invalid reply id: {}; must be 1-3", id))),
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&queries::config(deps)?),
        QueryMsg::State {
            addr,
        } => to_binary(&queries::state(deps, env, addr)?),
        QueryMsg::Share {
            addr,
        } => to_binary(&queries::share(deps, env, addr)?),
    }
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    let contract_version = get_contract_version(deps.storage)?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("previous_contract_name", &contract_version.contract)
        .add_attribute("previous_contract_version", &contract_version.version)
        .add_attribute("new_contract_name", CONTRACT_NAME)
        .add_attribute("new_contract_version", CONTRACT_VERSION))
}
