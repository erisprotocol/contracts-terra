use std::vec;

use cosmwasm_std::{to_binary, Decimal, QuerierResult, Uint128};
use eris::hub::{QueryMsg, StateResponse};

use super::helpers::err_unsupported_query;

#[derive(Default)]
pub(super) struct ErisQuerier {
    pub exchange_rate: Decimal,
}

impl ErisQuerier {
    pub fn handle_query(&self, _contract_addr: &str, query: QueryMsg) -> QuerierResult {
        match &query {
            QueryMsg::State {} => Ok(to_binary(&StateResponse {
                total_ustake: Uint128::zero(),
                total_utoken: Uint128::zero(),
                exchange_rate: self.exchange_rate,
                unlocked_coins: vec![],
                unbonding: Uint128::zero(),
                available: Uint128::zero(),
                tvl_utoken: Uint128::zero(),
            })
            .into())
            .into(),
            other_query => err_unsupported_query(other_query),
        }
    }
}
