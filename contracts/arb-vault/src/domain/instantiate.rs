use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, DepsMut, Env, Response, StdResult, SubMsg, SubMsgResponse, Uint128,
    WasmMsg,
};
use cw2::set_contract_version;
use cw20::MinterResponse;
use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;
use eris::arb_vault::{Config, InstantiateMsg, LsdConfig, ValidatedConfig};

use crate::{
    constants::{CONTRACT_NAME, CONTRACT_VERSION, INSTANTIATE_TOKEN_REPLY_ID},
    error::{ContractError, ContractResult, CustomResult},
    extensions::UtilizationMethodEx,
    state::{BalanceLocked, State},
};

//--------------------------------------------------------------------------------------------------
// Instantiation
//--------------------------------------------------------------------------------------------------

pub fn instantiate(deps: DepsMut, env: Env, msg: InstantiateMsg) -> ContractResult {
    let state = State::default();

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let lsds = msg
        .lsds
        .into_iter()
        .map(|lsd| lsd.validate(deps.api))
        .collect::<StdResult<Vec<LsdConfig<Addr>>>>()?;

    msg.utilization_method.validate()?;

    let config = ValidatedConfig {
        lp_addr: Addr::unchecked(""),
        unbond_time_s: msg.unbond_time_s,
        lsds,
        utoken: msg.utoken,
        utilization_method: msg.utilization_method,
    };

    state.owner.save(deps.storage, &deps.api.addr_validate(&msg.owner)?)?;
    state.config.save(deps.storage, &config)?;
    state.unbond_id.save(deps.storage, &0)?;
    state.fee_config.save(deps.storage, &msg.fee_config.validate(deps.api)?)?;

    state.update_whitelist(deps.storage, deps.api, msg.whitelist)?;

    state.balance_locked.save(
        deps.storage,
        &BalanceLocked {
            balance: Uint128::zero(),
        },
    )?;

    Ok(Response::new().add_submessage(SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Instantiate {
            admin: Some(msg.owner), // use the owner as admin for now; can be changed later by a `MsgUpdateAdmin`
            code_id: msg.cw20_code_id,
            msg: to_binary(&Cw20InstantiateMsg {
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
            label: "Eris Arb Vault LP Token".to_string(),
        }),
        INSTANTIATE_TOKEN_REPLY_ID,
    )))
}

pub fn register_lp_token(deps: DepsMut, response: SubMsgResponse) -> ContractResult {
    let state = State::default();

    let event = response
        .events
        .iter()
        .find(|event| event.ty == "instantiate")
        .ok_or(ContractError::CannotFindInstantiateEvent {})?;

    let contract_addr_str = &event
        .attributes
        .iter()
        .find(|attr| attr.key == "_contract_address" || attr.key == "_contract_addr")
        .ok_or(ContractError::CannotFindContractAddress {})?
        .value;

    let contract_addr = deps.api.addr_validate(contract_addr_str)?;
    state.config.update(deps.storage, |mut state| -> CustomResult<Config<Addr>> {
        state.lp_addr = contract_addr;
        Ok(state)
    })?;

    Ok(Response::new())
}
