use astroport::asset::native_asset;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, to_json_binary, Addr, CosmosMsg, StdResult, Uint128, WasmMsg};

use crate::arb_vault::ExecuteMsg;

#[cw_serde]
pub struct ArbVault(pub Addr);

impl ArbVault {
    pub fn deposit_msg(
        &self,
        denom: impl Into<String>,
        amount: u128,
        receiver: Option<String>,
    ) -> StdResult<CosmosMsg> {
        let denom_str: String = denom.into();
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            msg: to_json_binary(&ExecuteMsg::Deposit {
                asset: native_asset(denom_str.clone(), Uint128::new(amount)),
                receiver,
            })?,
            funds: vec![coin(amount, denom_str)],
        }))
    }
}
