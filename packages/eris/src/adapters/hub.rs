use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, to_binary, Addr, CosmosMsg, StdResult, WasmMsg};

use crate::hub::ExecuteMsg;

#[cw_serde]
pub struct Hub(pub Addr);

impl Hub {
    pub fn bond_msg(
        &self,
        denom: impl Into<String>,
        amount: u128,
        receiver: Option<String>,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            msg: to_binary(&ExecuteMsg::Bond {
                receiver,
            })?,
            funds: vec![coin(amount, denom)],
        }))
    }
}
