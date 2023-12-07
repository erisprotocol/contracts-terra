use astroport::asset::Asset;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Addr, CosmosMsg, Decimal, StdResult, WasmMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct WhiteWhale(pub Addr);

#[cw_serde]
pub enum ExecuteMsg {
    Claim {},

    Deposit {
        /// The address of the pair to deposit.
        pair_address: String,
        /// The assets to deposit into the pair.
        assets: [Asset; 2],
        /// The
        slippage_tolerance: Option<Decimal>,
        /// The amount of time in seconds to unbond tokens for when incentivizing.
        unbonding_duration: u64,
    },
}

impl WhiteWhale {
    pub fn claim_msg(&self) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            funds: vec![],
            msg: to_binary(&ExecuteMsg::Claim {})?,
        }))
    }

    pub fn front_end_deposit(
        &self,
        pair_address: String,
        assets: [Asset; 2],
        slippage_tolerance: Option<Decimal>,
        unbonding_duration: u64,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            funds: vec![],
            msg: to_binary(&ExecuteMsg::Deposit {
                pair_address,
                assets,
                slippage_tolerance,
                unbonding_duration,
            })?,
        }))
    }
}
