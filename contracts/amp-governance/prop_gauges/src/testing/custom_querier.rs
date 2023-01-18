use std::collections::HashMap;
use std::ops::Mul;

use cosmwasm_schema::serde::Serialize;
use cosmwasm_std::testing::{BankQuerier, StakingQuerier, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Coin, Decimal, Empty, Querier, QuerierResult, QueryRequest,
    SystemError, Uint128, WasmQuery,
};
use cw20::Cw20QueryMsg;
use eris::voting_escrow::{LockInfoResponse, VotingPowerResponse};

use super::cw20_querier::Cw20Querier;
use super::helpers::err_unsupported_query;

#[derive(Default)]
pub(super) struct CustomQuerier {
    pub cw20_querier: Cw20Querier,
    pub bank_querier: BankQuerier,
    pub staking_querier: StakingQuerier,

    pub vp: HashMap<String, LockInfoResponse>,
}

impl Querier for CustomQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        let request: QueryRequest<_> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
                .into()
            },
        };
        self.handle_query(&request)
    }
}

impl CustomQuerier {
    #[allow(dead_code)]
    pub fn set_cw20_balance(&mut self, token: &str, user: &str, balance: u128) {
        match self.cw20_querier.balances.get_mut(token) {
            Some(contract_balances) => {
                contract_balances.insert(user.to_string(), balance);
            },
            None => {
                let mut contract_balances: HashMap<String, u128> = HashMap::default();
                contract_balances.insert(user.to_string(), balance);
                self.cw20_querier.balances.insert(token.to_string(), contract_balances);
            },
        };
    }

    #[allow(dead_code)]
    pub fn set_cw20_total_supply(&mut self, token: &str, total_supply: u128) {
        self.cw20_querier.total_supplies.insert(token.to_string(), total_supply);
    }

    #[allow(dead_code)]
    pub fn set_bank_balances(&mut self, balances: &[Coin]) {
        self.bank_querier = BankQuerier::new(&[(MOCK_CONTRACT_ADDR, balances)])
    }

    pub fn set_lock(&mut self, user: impl Into<String>, fixed: u128, dynamic: u128) {
        self.vp.insert(
            user.into(),
            LockInfoResponse {
                amount: Uint128::zero(),
                coefficient: Decimal::zero(),
                start: 0,
                end: 10,
                slope: Uint128::new(1),
                fixed_amount: Uint128::new(fixed),
                voting_power: Uint128::new(dynamic),
            },
        );
    }

    // pub fn set_staking_delegations(&mut self, delegations: &[Delegation]) {
    //     let fds = delegations
    //         .iter()
    //         .map(|d| FullDelegation {
    //             delegator: Addr::unchecked(MOCK_CONTRACT_ADDR),
    //             validator: d.validator.clone(),
    //             amount: Coin::new(d.amount, "uluna"),
    //             can_redelegate: Coin::new(0, "uluna"),
    //             accumulated_rewards: vec![],
    //         })
    //         .collect::<Vec<_>>();

    //     self.staking_querier = StakingQuerier::new("uluna", &[], &fds);
    // }

    pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match request {
            QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr,
                msg,
            }) => {
                if let Ok(query) = from_binary::<Cw20QueryMsg>(msg) {
                    return self.cw20_querier.handle_query(contract_addr, query);
                }

                if let Ok(query) = from_binary::<eris::voting_escrow::QueryMsg>(msg) {
                    return self.handle_vp_query(contract_addr, query);
                }

                err_unsupported_query(msg)
            },

            QueryRequest::Bank(query) => self.bank_querier.query(query),

            QueryRequest::Staking(query) => self.staking_querier.query(query),

            _ => err_unsupported_query(request),
        }
    }

    pub fn to_result<T>(&self, val: T) -> QuerierResult
    where
        T: Serialize + Sized,
    {
        Ok(to_binary(&val).into()).into()
    }

    fn handle_vp_query(
        &self,
        _contract_addr: &str,
        query: eris::voting_escrow::QueryMsg,
    ) -> QuerierResult {
        match query {
            eris::voting_escrow::QueryMsg::CheckVotersAreBlacklisted {
                ..
            } => todo!(),
            eris::voting_escrow::QueryMsg::BlacklistedVoters {
                ..
            } => todo!(),
            eris::voting_escrow::QueryMsg::Balance {
                ..
            } => todo!(),
            eris::voting_escrow::QueryMsg::TokenInfo {} => todo!(),
            eris::voting_escrow::QueryMsg::MarketingInfo {} => todo!(),
            eris::voting_escrow::QueryMsg::DownloadLogo {} => todo!(),
            eris::voting_escrow::QueryMsg::TotalVamp {} => todo!(),
            eris::voting_escrow::QueryMsg::TotalVampAt {
                ..
            } => todo!(),
            eris::voting_escrow::QueryMsg::TotalVampAtPeriod {
                period,
            } => {
                let mut vamp = Uint128::zero();

                for x in self.vp.values() {
                    if period >= x.start {
                        let diff = period - x.start;
                        vamp = vamp + x.fixed_amount + x.voting_power
                            - x.slope.mul(Uint128::new(diff.into()));
                    }
                }

                self.to_result(VotingPowerResponse {
                    vamp,
                })
            },
            eris::voting_escrow::QueryMsg::UserVamp {
                ..
            } => todo!(),
            eris::voting_escrow::QueryMsg::UserVampAt {
                ..
            } => todo!(),
            eris::voting_escrow::QueryMsg::UserVampAtPeriod {
                ..
            } => todo!(),
            eris::voting_escrow::QueryMsg::LockInfo {
                user,
            } => self.to_result(self.vp.get(&user)),
            eris::voting_escrow::QueryMsg::UserDepositAtHeight {
                ..
            } => todo!(),
            eris::voting_escrow::QueryMsg::Config {} => todo!(),
        }
    }
}
