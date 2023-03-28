use crate::constants::{CONTRACT_NAME, CONTRACT_VERSION, INSTANTIATE_TOKEN_REPLY_ID};
use crate::domain;
use crate::domain::callback::handle_callback;
use crate::domain::config::execute_update_config;
use crate::domain::execute::{
    execute_arbitrage, execute_deposit, execute_unbond_liquidity, execute_withdraw_liquidity,
    execute_withdraw_unbonded, execute_withdraw_unbonding_immediate, receive_cw20,
};
use crate::domain::ownership::{claim_ownership, drop_ownership_proposal, propose_new_owner};
use crate::error::{ContractError, ContractResult, CustomResult};
use crate::query::{
    query_config, query_exchange_rates, query_state, query_takeable, query_unbond_requests,
    query_user_info,
};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
};
use cw2::{get_contract_version, set_contract_version};

use eris::arb_vault::InstantiateMsg;
use eris::arb_vault::{ExecuteMsg, MigrateMsg, QueryMsg};
use eris::helper::unwrap_reply;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult {
    domain::instantiate::instantiate(deps, env, msg)
}

#[entry_point]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> ContractResult {
    match reply.id {
        INSTANTIATE_TOKEN_REPLY_ID => {
            domain::instantiate::register_lp_token(deps, unwrap_reply(reply)?)
        },
        id => Err(ContractError::InvalidReplyId(id)),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // Allowed by Execute whitelist
        ExecuteMsg::ExecuteArbitrage {
            msg,
            result_token,
            wanted_profit,
        } => execute_arbitrage(deps, env, info, msg, result_token, wanted_profit),

        ExecuteMsg::WithdrawFromLiquidStaking {
            names,
        } => execute_withdraw_liquidity(deps, env, info, names),

        ExecuteMsg::UnbondFromLiquidStaking {
            names,
        } => execute_unbond_liquidity(deps, env, info, names),

        // User actions
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::Deposit {
            asset,
            receiver,
        } => execute_deposit(deps, env, info, asset, receiver),
        ExecuteMsg::WithdrawUnbonded {
            ..
        } => execute_withdraw_unbonded(deps, env, info),
        ExecuteMsg::WithdrawImmediate {
            id,
        } => execute_withdraw_unbonding_immediate(deps, env, info, id),

        // ExecuteMsg::Swap {
        //     offer_asset,
        //     ask_asset_info,
        //     belief_price,
        //     max_spread,
        //     to,
        // } => domain::swap::execute_swap_native(
        //     deps,
        //     env,
        //     info,
        //     offer_asset,
        //     ask_asset_info,
        //     belief_price,
        //     max_spread,
        //     to,
        // ),
        // Allowed by Owner
        ExecuteMsg::UpdateConfig {
            ..
        } => execute_update_config(deps, env, info, msg),

        ExecuteMsg::ProposeNewOwner {
            owner,
            expires_in,
        } => propose_new_owner(deps, info, env, owner, expires_in),
        ExecuteMsg::DropOwnershipProposal {} => drop_ownership_proposal(deps, info),
        ExecuteMsg::ClaimOwnership {} => claim_ownership(deps, info, env),
        ExecuteMsg::Callback(msg) => handle_callback(deps, env, info, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> CustomResult<Binary> {
    let res = match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?)?,
        QueryMsg::State {
            details,
        } => to_binary(&query_state(deps, env, details)?)?,
        QueryMsg::UserInfo {
            address,
        } => to_binary(&query_user_info(deps, env, address)?)?,
        QueryMsg::Takeable {
            wanted_profit,
        } => to_binary(&query_takeable(deps, env, wanted_profit)?)?,

        QueryMsg::UnbondRequests {
            address,
            limit,
            start_after,
        } => to_binary(&query_unbond_requests(deps, env, address, start_after, limit)?)?,

        QueryMsg::ExchangeRates {
            start_after_d,
            limit,
        } => to_binary(&query_exchange_rates(deps, env, start_after_d, limit)?)?,
    };
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let contract_version = get_contract_version(deps.storage)?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("previous_contract_name", &contract_version.contract)
        .add_attribute("previous_contract_version", &contract_version.version)
        .add_attribute("new_contract_name", CONTRACT_NAME)
        .add_attribute("new_contract_version", CONTRACT_VERSION))
}
