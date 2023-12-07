use astroport::asset::{Asset, AssetInfo};
use cosmwasm_std::{to_binary, Addr, Coin, CosmosMsg, Decimal, QuerierWrapper, StdResult, WasmMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::compound_proxy::{ExecuteMsg, LpStateResponse, QueryMsg, SupportsSwapResponse};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Compounder(pub Addr);

impl Compounder {
    pub fn query_lp_state(
        &self,
        querier: &QuerierWrapper,
        lp_addr: String,
    ) -> StdResult<LpStateResponse> {
        querier.query_wasm_smart(
            self.0.to_string(),
            &QueryMsg::GetLpState {
                lp_addr,
            },
        )
    }

    pub fn query_support_swap(
        &self,
        querier: &QuerierWrapper,
        from: AssetInfo,
        to: AssetInfo,
    ) -> StdResult<bool> {
        let res: SupportsSwapResponse = querier.query_wasm_smart(
            self.0.to_string(),
            &QueryMsg::SupportsSwap {
                from,
                to,
            },
        )?;
        Ok(res.suppored)
    }

    pub fn compound_msg(
        &self,
        rewards: Vec<Asset>,
        mut funds: Vec<Coin>,
        no_swap: Option<bool>,
        slippage_tolerance: Option<Decimal>,
        lp_token: &Addr,
        receiver: Option<String>,
    ) -> StdResult<CosmosMsg> {
        funds.sort_by(|a, b| a.denom.cmp(&b.denom));
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            msg: to_binary(&ExecuteMsg::Compound {
                lp_token: lp_token.to_string(),
                rewards,
                no_swap,
                receiver,
                slippage_tolerance,
            })?,
            funds,
        }))
    }

    pub fn multi_swap_msg(
        &self,
        rewards: Vec<Asset>,
        into: AssetInfo,
        mut funds: Vec<Coin>,
        receiver: Option<String>,
    ) -> StdResult<CosmosMsg> {
        funds.sort_by(|a, b| a.denom.cmp(&b.denom));
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            msg: to_binary(&ExecuteMsg::MultiSwap {
                assets: rewards,
                into,
                receiver,
            })?,
            funds,
        }))
    }
}
