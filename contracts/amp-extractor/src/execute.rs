use cosmwasm_std::{
    to_json_binary, Addr, CosmosMsg, Decimal, DepsMut, Env, Event, Response, StdError, StdResult,
    SubMsg, SubMsgResponse, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, MinterResponse};
use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;
use eris::DecimalCheckedOps;

use eris::amp_extractor::{ExtractConfig, InstantiateMsg};

use crate::constants::assert_valid_yield_extract;
use crate::helpers::{query_cw20_balance, query_cw20_total_supply, query_exchange_rate};
use crate::math::{compute_mint_amount, compute_withdraw_amount};
use crate::state::State;

const CONTRACT_NAME: &str = "eris-hub";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

//--------------------------------------------------------------------------------------------------
// Instantiation
//--------------------------------------------------------------------------------------------------

pub fn instantiate(deps: DepsMut, env: Env, msg: InstantiateMsg) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let state = State::default();

    assert_valid_yield_extract(&msg.yield_extract_p)?;

    state.owner.save(deps.storage, &deps.api.addr_validate(&msg.owner)?)?;
    state.last_exchange_rate.save(deps.storage, &Decimal::zero())?;
    state.stake_extracted.save(deps.storage, &Uint128::zero())?;
    state.stake_harvested.save(deps.storage, &Uint128::zero())?;
    state.stake_token.save(deps.storage, &deps.api.addr_validate(&msg.stake_token)?)?;
    state.extract_config.save(
        deps.storage,
        &ExtractConfig {
            yield_extract_addr: deps.api.addr_validate(&msg.yield_extract_addr)?,
            yield_extract_p: msg.yield_extract_p,
            interface: msg.interface,
            hub_contract: deps.api.addr_validate(&msg.hub_contract)?,
        },
    )?;

    Ok(Response::new().add_submessage(SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Instantiate {
            admin: Some(msg.owner), // use the owner as admin for now; can be changed later by a `MsgUpdateAdmin`
            code_id: msg.cw20_code_id,
            msg: to_json_binary(&Cw20InstantiateMsg {
                name: msg.name,
                symbol: msg.symbol,
                decimals: msg.decimals,
                initial_balances: vec![],
                mint: Some(MinterResponse {
                    minter: env.contract.address.into(),
                    cap: None,
                }),
                marketing: None,
            })?,
            funds: vec![],
            label: msg.label,
        }),
        1,
    )))
}

pub fn register_lp_token(deps: DepsMut, response: SubMsgResponse) -> StdResult<Response> {
    let state = State::default();

    let event = response
        .events
        .iter()
        .find(|event| event.ty == "instantiate")
        .ok_or_else(|| StdError::generic_err("cannot find `instantiate` event"))?;

    let contract_addr_str = &event
        .attributes
        .iter()
        .find(|attr| attr.key == "_contract_address")
        .ok_or_else(|| StdError::generic_err("cannot find `_contract_address` attribute"))?
        .value;

    let contract_addr = deps.api.addr_validate(contract_addr_str)?;
    state.lp_token.save(deps.storage, &contract_addr)?;

    Ok(Response::new())
}

//--------------------------------------------------------------------------------------------------
// Harvesting logic
//--------------------------------------------------------------------------------------------------

pub fn harvest(mut deps: DepsMut, env: Env, user: Addr) -> StdResult<Response> {
    let state = State::default();
    let stake_token = state.stake_token.load(deps.storage)?;
    let extract_config = state.extract_config.load(deps.storage)?;

    let _stake_available = extract(&mut deps, env, &state, None)?;
    let stake_extracted = state.stake_extracted.load(deps.storage)?;
    let stake_harvested = state.stake_harvested.load(deps.storage)?;

    state.stake_extracted.save(deps.storage, &Uint128::zero())?;
    state.stake_harvested.save(deps.storage, &stake_harvested.checked_add(stake_extracted)?)?;

    // refund remaining stake token
    let harvest_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: stake_token.into(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            amount: stake_extracted,
            recipient: extract_config.yield_extract_addr.to_string(),
        })?,
        funds: vec![],
    });

    let event = Event::new("erisextractor/harvested")
        .add_attribute("user", user)
        .add_attribute("stake_extracted", stake_extracted);

    Ok(Response::new()
        .add_message(harvest_msg)
        .add_event(event)
        .add_attribute("action", "erisextractor/harvest"))
}

//--------------------------------------------------------------------------------------------------
// Deposit / Withdraw logic
//--------------------------------------------------------------------------------------------------

pub fn withdraw(
    mut deps: DepsMut,
    env: Env,
    user: Addr,
    lp_amount: Uint128,
) -> StdResult<Response> {
    let state = State::default();

    let stake_token = state.stake_token.load(deps.storage)?;
    let lp_token = state.lp_token.load(deps.storage)?;
    let lp_token_supply = query_cw20_total_supply(&deps.querier, &lp_token)?;
    let stake_available = extract(&mut deps, env, &state, None)?;

    let stake_withdraw_amount =
        compute_withdraw_amount(lp_token_supply, lp_amount, stake_available);

    // burn deposited lp token
    let burn_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: lp_token.into(),
        msg: to_json_binary(&Cw20ExecuteMsg::Burn {
            amount: lp_amount,
        })?,
        funds: vec![],
    });

    // refund remaining stake token
    let refund_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: stake_token.into(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            amount: stake_withdraw_amount,
            recipient: user.to_string(),
        })?,
        funds: vec![],
    });

    let event = Event::new("erisextractor/withdrawn")
        .add_attribute("user", user)
        .add_attribute("stake_withdrawn", stake_withdraw_amount)
        .add_attribute("lp_burned", lp_amount);

    Ok(Response::new()
        .add_messages(vec![burn_msg, refund_msg])
        .add_event(event)
        .add_attribute("action", "erisextractor/withdraw"))
}

pub fn deposit(
    mut deps: DepsMut,
    env: Env,
    receiver: Addr,
    stake_deposited: Uint128,
) -> StdResult<Response> {
    let state = State::default();

    let lp_token = state.lp_token.load(deps.storage)?;
    let lp_token_supply = query_cw20_total_supply(&deps.querier, &lp_token)?;
    let stake_available = extract(&mut deps, env, &state, Some(stake_deposited))?;

    let lp_mint_amount = compute_mint_amount(lp_token_supply, stake_deposited, stake_available);

    let mint_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: lp_token.into(),
        msg: to_json_binary(&Cw20ExecuteMsg::Mint {
            recipient: receiver.to_string(),
            amount: lp_mint_amount,
        })?,
        funds: vec![],
    });

    let event = Event::new("erisextractor/deposited")
        .add_attribute("receiver", receiver)
        .add_attribute("stake_deposited", stake_deposited)
        .add_attribute("lp_minted", lp_mint_amount);

    Ok(Response::new()
        .add_message(mint_msg)
        .add_event(event)
        .add_attribute("action", "erisextractor/deposit"))
}

pub fn extract(
    deps: &mut DepsMut,
    env: Env,
    state: &State,
    offset_balance: Option<Uint128>,
) -> StdResult<Uint128> {
    let extract_config: ExtractConfig = state.extract_config.load(deps.storage)?;
    let stake_token = state.stake_token.load(deps.storage)?;
    let stake_extracted = state.stake_extracted.load(deps.storage)?;
    let last_exchange_rate = state.last_exchange_rate.load(deps.storage)?;

    let current_exchange_rate =
        query_exchange_rate(&deps.querier, extract_config.interface, &extract_config.hub_contract)?;

    let mut stake_in_contract =
        query_cw20_balance(&deps.querier, &stake_token, &env.contract.address)?;

    if let Some(offset_balance) = offset_balance {
        // if we received some stake balance we need to ignore it for extraction.
        stake_in_contract = stake_in_contract.checked_sub(offset_balance)?;
    }

    let stake_available = stake_in_contract.checked_sub(stake_extracted)?;

    if current_exchange_rate.le(&last_exchange_rate) {
        // if the current rate is lower or equal to the last exchange rate nothing will be extracted
        // it is expected that exchange_rate will only increase - slashings ignored / nothing extracted until it is higher again.
        return Ok(stake_available);
    }

    if last_exchange_rate.is_zero() {
        state.last_exchange_rate.save(deps.storage, &current_exchange_rate)?;
        return Ok(stake_available);
    }

    // no check needed, as we checked for "le" already. current_exchange_rate is also not zero
    let exchange_rate_diff = (current_exchange_rate - last_exchange_rate) / current_exchange_rate;

    let stake_to_extract = exchange_rate_diff
        .checked_mul(extract_config.yield_extract_p)?
        .checked_mul_uint(stake_available)?;

    let stake_extracted_new = stake_extracted.checked_add(stake_to_extract)?;

    state.stake_extracted.save(deps.storage, &stake_extracted_new)?;
    state.last_exchange_rate.save(deps.storage, &current_exchange_rate)?;

    let stake_available_new = stake_in_contract.checked_sub(stake_extracted_new)?;

    Ok(stake_available_new)
}

//--------------------------------------------------------------------------------------------------
// Ownership and management logics
//--------------------------------------------------------------------------------------------------

pub fn transfer_ownership(deps: DepsMut, sender: Addr, new_owner: String) -> StdResult<Response> {
    let state = State::default();

    state.assert_owner(deps.storage, &sender)?;
    state.new_owner.save(deps.storage, &deps.api.addr_validate(&new_owner)?)?;

    Ok(Response::new().add_attribute("action", "erisextractor/transfer_ownership"))
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

    let event = Event::new("erisextractor/ownership_transferred")
        .add_attribute("new_owner", new_owner)
        .add_attribute("previous_owner", previous_owner);

    Ok(Response::new().add_event(event).add_attribute("action", "erisextractor/transfer_ownership"))
}

pub fn update_config(
    deps: DepsMut,
    sender: Addr,
    yield_extract_addr: Option<String>,
) -> StdResult<Response> {
    let state = State::default();

    state.assert_owner(deps.storage, &sender)?;

    let mut extract_config = state.extract_config.load(deps.storage)?;

    if let Some(yield_extract_addr) = yield_extract_addr {
        extract_config.yield_extract_addr = deps.api.addr_validate(&yield_extract_addr)?;
    }

    state.extract_config.save(deps.storage, &extract_config)?;

    Ok(Response::new().add_attribute("action", "erisextractor/update_config"))
}
