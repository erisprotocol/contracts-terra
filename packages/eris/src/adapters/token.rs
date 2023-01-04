use astroport::querier::{query_supply, query_token_balance};
use cosmwasm_std::{to_binary, Addr, CosmosMsg, QuerierWrapper, StdResult, Uint128, WasmMsg};
use cw20::{Cw20ExecuteMsg, MinterResponse};
use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::astroport_farm::TokenInit;

impl TokenInit {
    pub fn instantiate(&self, owner: String, contract: Addr) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Instantiate {
            admin: Some(owner), // use the owner as admin for now; can be changed later by a `MsgUpdateAdmin`
            code_id: self.cw20_code_id,
            msg: to_binary(&Cw20InstantiateMsg {
                name: self.name.clone(),
                symbol: self.symbol.clone(),
                decimals: self.decimals,
                initial_balances: vec![],
                mint: Some(MinterResponse {
                    minter: contract.into(),
                    cap: None,
                }),
                marketing: None,
            })?,
            funds: vec![],
            label: "Eris Amplified Compounder Token".to_string(),
        }))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Token(pub Addr);

impl Token {
    pub fn mint(&self, amount: Uint128, receiver: Addr) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: receiver.to_string(),
                amount,
            })?,
            funds: vec![],
        }))
    }

    pub fn burn(&self, amount: Uint128) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Burn {
                amount,
            })?,
            funds: vec![],
        }))
    }

    pub fn query_amount(&self, querier: &QuerierWrapper, account: Addr) -> StdResult<Uint128> {
        query_token_balance(querier, self.0.to_string(), account)
    }

    pub fn query_supply(&self, querier: &QuerierWrapper) -> StdResult<Uint128> {
        query_supply(querier, self.0.to_string())
    }
}
