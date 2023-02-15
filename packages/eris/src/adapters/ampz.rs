use astroport::asset::Asset;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Addr, Coin, CosmosMsg, StdResult, WasmMsg};

use crate::ampz::ExecuteMsg;

#[cw_serde]
pub struct Ampz(pub Addr);

impl Ampz {
    pub fn deposit(&self, assets: Vec<Asset>, funds: Vec<Coin>) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            msg: to_binary(&ExecuteMsg::Deposit {
                assets,
            })?,
            funds,
        }))
    }
}
