use astroport::generator::{Cw20HookMsg, ExecuteMsg, PendingTokenResponse, QueryMsg};
use cosmwasm_std::{to_binary, Addr, CosmosMsg, QuerierWrapper, StdResult, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Generator(pub Addr);

impl Generator {
    pub fn query_pending_token(
        &self,
        querier: &QuerierWrapper,
        lp_token: &Addr,
        staker: &Addr,
    ) -> StdResult<PendingTokenResponse> {
        querier.query_wasm_smart(
            self.0.to_string(),
            &QueryMsg::PendingToken {
                lp_token: lp_token.to_string(),
                user: staker.to_string(),
            },
        )
    }

    pub fn query_deposit(
        &self,
        querier: &QuerierWrapper,
        lp_token: &Addr,
        staker: &Addr,
    ) -> StdResult<Uint128> {
        querier.query_wasm_smart(
            self.0.to_string(),
            &QueryMsg::Deposit {
                lp_token: lp_token.to_string(),
                user: staker.to_string(),
            },
        )
    }

    pub fn deposit_msg(&self, lp_token: String, amount: Uint128) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: lp_token,
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: self.0.to_string(),
                amount,
                msg: to_binary(&Cw20HookMsg::Deposit {})?,
            })?,
        }))
    }

    pub fn withdraw_msg(&self, lp_token: String, amount: Uint128) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            funds: vec![],
            msg: to_binary(&ExecuteMsg::Withdraw {
                lp_token,
                amount,
            })?,
        }))
    }

    pub fn claim_rewards_msg(&self, lp_tokens: Vec<String>) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            msg: to_binary(&ExecuteMsg::ClaimRewards {
                lp_tokens,
            })?,
            funds: vec![],
        }))
    }
}
