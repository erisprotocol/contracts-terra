use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdResult, WasmMsg};
use eris::hub::ClaimType;

use crate::{
    error::{ContractError, ContractResult},
    state::State,
};

#[cw_serde]
pub enum ClaimExecuteMsg {
    Claim {},
}

impl ClaimExecuteMsg {
    pub fn into_msg(&self, contract_addr: String) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            msg: to_binary(&self)?,
            funds: vec![],
        }))
    }
}

pub fn exec_claim(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    claims: Vec<ClaimType>,
) -> ContractResult {
    let state = State::default();
    state.assert_owner(deps.storage, &info.sender)?;

    if claims.is_empty() {
        return Err(ContractError::NoClaimsProvided {});
    }

    let claim_msgs = claims
        .into_iter()
        .map(|claim| {
            Ok(match claim {
                ClaimType::Default(contract_addr) => {
                    deps.api.addr_validate(&contract_addr)?;
                    ClaimExecuteMsg::Claim {}.into_msg(contract_addr)?
                },
            })
        })
        .collect::<StdResult<Vec<CosmosMsg>>>()?;

    Ok(Response::new().add_messages(claim_msgs).add_attribute("action", "erishub/exec_claim"))
}
