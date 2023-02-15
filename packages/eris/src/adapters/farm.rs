use astroport::asset::Asset;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Addr, Coin, CosmosMsg, StdResult, WasmMsg};

use crate::astroport_farm::ExecuteMsg;

#[cw_serde]
pub struct Farm(pub Addr);

impl Farm {
    pub fn bond_assets_msg(
        &self,
        assets: Vec<Asset>,
        mut funds: Vec<Coin>,
        receiver: Option<String>,
    ) -> StdResult<CosmosMsg> {
        funds.sort_by(|a, b| a.denom.cmp(&b.denom));
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            msg: to_binary(&ExecuteMsg::BondAssets {
                assets,
                minimum_receive: None,
                no_swap: None,
                slippage_tolerance: None,
                receiver,
            })?,
            funds,
        }))
    }
}
