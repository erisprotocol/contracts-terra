use std::collections::HashMap;
use std::str::FromStr;

use cosmwasm_std::testing::{BankQuerier, StakingQuerier, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    coin, from_binary, from_slice, to_json_binary, Coin, Decimal, Empty, Querier, QuerierResult,
    QueryRequest, SystemError, SystemResult, Timestamp, Uint128, WasmQuery,
};
use cw20::Cw20QueryMsg;
use stader::state::UndelegationInfo;

use crate::lsds::stader::{BatchUndelegationRecord, QueryBatchUndelegationResponse};

use super::cw20_querier::Cw20Querier;
use super::helpers::err_unsupported_query;

#[derive(Default)]
pub(super) struct CustomQuerier {
    pub cw20_querier: Cw20Querier,
    pub bank_querier: BankQuerier,
    pub staking_querier: StakingQuerier,
    pub unbonding_amount: Uint128,
    pub unbonding_amount_eris: Option<Uint128>,
    pub withdrawable_amount: Uint128,
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

    pub fn set_cw20_total_supply(&mut self, token: &str, total_supply: u128) {
        self.cw20_querier.total_supplies.insert(token.to_string(), total_supply);
    }

    pub fn set_bank_balances(&mut self, balances: &[Coin]) {
        self.bank_querier = BankQuerier::new(&[(MOCK_CONTRACT_ADDR, balances)])
    }

    pub fn with_unbonding(&mut self, amount: Uint128) {
        self.unbonding_amount = amount;
    }

    pub fn with_unbonding_eris(&mut self, amount: Uint128) {
        self.unbonding_amount_eris = Some(amount);
    }

    pub fn with_withdrawable(&mut self, amount: Uint128) {
        self.withdrawable_amount = amount;
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

                if contract_addr == "prism" {
                    return match from_binary(msg).unwrap() {
                        prism::hub::QueryMsg::UnbondRequests {
                            address,
                            ..
                        } => SystemResult::Ok(
                            to_json_binary(&prism::hub::UnbondRequestsResponse {
                                address,
                                requests: vec![(1u64, self.unbonding_amount)],
                            })
                            .into(),
                        ),
                        prism::hub::QueryMsg::WithdrawableUnbonded {
                            ..
                        } => SystemResult::Ok(
                            to_json_binary(&prism::hub::WithdrawableUnbondedResponse {
                                withdrawable: self.withdrawable_amount,
                            })
                            .into(),
                        ),
                        prism::hub::QueryMsg::Config {
                            ..
                        } => SystemResult::Ok(
                            to_json_binary(&prism::hub::StateResponse {
                                exchange_rate: Decimal::one(),
                                total_bond_amount: Uint128::zero(),
                                last_index_modification: 0,
                                actual_unbonded_amount: Uint128::zero(),
                                last_unbonded_time: 0,
                                last_processed_batch: 0,
                                principle_balance_before_exchange_update: Uint128::zero(),
                                prev_hub_balance: Uint128::zero(),
                            })
                            .into(),
                        ),
                        prism::hub::QueryMsg::AllHistory {
                            start_from,
                            ..
                        } => SystemResult::Ok(
                            to_json_binary(&prism::hub::AllHistoryResponse {
                                history: vec![prism::hub::UnbondHistory {
                                    amount: Uint128::zero(),
                                    batch_id: start_from.map(|val| val + 1).unwrap_or_default(),
                                    time: 0,
                                    applied_exchange_rate: Decimal::one(),
                                    withdraw_rate: Decimal::one(),
                                    released: false,
                                }],
                            })
                            .into(),
                        ),
                        prism::hub::QueryMsg::State {} => SystemResult::Ok(
                            to_json_binary(&prism::hub::StateResponse {
                                exchange_rate: Decimal::one(),
                                total_bond_amount: Uint128::zero(),
                                last_index_modification: 0,
                                principle_balance_before_exchange_update: Uint128::zero(),
                                prev_hub_balance: Uint128::zero(),
                                actual_unbonded_amount: Uint128::zero(),
                                last_unbonded_time: 0,
                                last_processed_batch: 0,
                            })
                            .into(),
                        ),
                        _ => err_unsupported_query(msg),
                    };
                } else if contract_addr == "eris" {
                    return match from_binary(msg).unwrap() {
                        eris::hub::QueryMsg::PendingBatch {} => SystemResult::Ok(
                            to_json_binary(&eris::hub::PendingBatch {
                                id: 3,
                                ustake_to_burn: Uint128::from(1000u128),
                                est_unbond_start_time: 123,
                            })
                            .into(),
                        ),
                        eris::hub::QueryMsg::PreviousBatch(id) => SystemResult::Ok(
                            to_json_binary(&eris::hub::Batch {
                                id,
                                reconciled: id < 2,
                                total_shares: Uint128::from(1000u128),
                                uluna_unclaimed: Uint128::from(1100u128),
                                est_unbond_end_time: 100,
                            })
                            .into(),
                        ),
                        eris::hub::QueryMsg::UnbondRequestsByUser {
                            ..
                        } => {
                            let mut res = vec![
                                eris::hub::UnbondRequestsByUserResponseItem {
                                    id: 1,
                                    shares: self.withdrawable_amount,
                                },
                                eris::hub::UnbondRequestsByUserResponseItem {
                                    id: 2,
                                    shares: self.unbonding_amount,
                                },
                            ];

                            if let Some(unbonding_amount_eris) = self.unbonding_amount_eris {
                                res.push(eris::hub::UnbondRequestsByUserResponseItem {
                                    id: 3,
                                    shares: unbonding_amount_eris,
                                })
                            }

                            SystemResult::Ok(to_json_binary(&res).into())
                        },
                        eris::hub::QueryMsg::State {} => SystemResult::Ok(
                            to_json_binary(&eris::hub::StateResponse {
                                total_ustake: Uint128::from(1000u128),
                                total_uluna: Uint128::from(1100u128),
                                exchange_rate: Decimal::from_str("1.1").unwrap(),
                                unlocked_coins: vec![],
                                unbonding: Uint128::new(0),
                                available: Uint128::new(0),
                                tvl_uluna: Uint128::new(1100),
                            })
                            .into(),
                        ),
                        _ => err_unsupported_query(msg),
                    };
                } else if contract_addr == "backbone" {
                    return match from_binary(msg).unwrap() {
                        steak::hub::QueryMsg::PendingBatch {} => SystemResult::Ok(
                            to_json_binary(&steak::hub::PendingBatch {
                                id: 3,
                                usteak_to_burn: Uint128::from(1000u128),
                                est_unbond_start_time: 123,
                            })
                            .into(),
                        ),
                        steak::hub::QueryMsg::PreviousBatch(id) => SystemResult::Ok(
                            to_json_binary(&steak::hub::Batch {
                                id,
                                reconciled: id < 2,
                                total_shares: Uint128::from(1000u128),
                                amount_unclaimed: Uint128::from(1000u128),
                                est_unbond_end_time: 100,
                            })
                            .into(),
                        ),
                        steak::hub::QueryMsg::UnbondRequestsByUser {
                            ..
                        } => SystemResult::Ok(
                            to_json_binary(&vec![
                                steak::hub::UnbondRequestsByUserResponseItem {
                                    id: 1,
                                    shares: self.withdrawable_amount,
                                },
                                steak::hub::UnbondRequestsByUserResponseItem {
                                    id: 2,
                                    shares: self.unbonding_amount,
                                },
                            ])
                            .into(),
                        ),
                        steak::hub::QueryMsg::State {} => SystemResult::Ok(
                            to_json_binary(&steak::hub::StateResponse {
                                total_usteak: Uint128::from(1000u128),
                                total_native: Uint128::from(1000u128),
                                exchange_rate: Decimal::one(),
                                unlocked_coins: vec![],
                            })
                            .into(),
                        ),
                        _ => err_unsupported_query(msg),
                    };
                } else if contract_addr == "stader" {
                    return match from_binary(msg).unwrap() {
                        stader::msg::QueryMsg::GetUserUndelegationRecords {
                            ..
                        } => SystemResult::Ok(
                            to_json_binary(&vec![
                                UndelegationInfo {
                                    batch_id: 0u64,
                                    token_amount: self.withdrawable_amount,
                                },
                                UndelegationInfo {
                                    batch_id: 1u64,
                                    token_amount: self.unbonding_amount,
                                },
                            ])
                            .into(),
                        ),
                        stader::msg::QueryMsg::BatchUndelegation {
                            batch_id,
                        } => SystemResult::Ok(
                            to_json_binary(&QueryBatchUndelegationResponse {
                                batch: Some(BatchUndelegationRecord {
                                    undelegated_tokens: Uint128::from(0u128),
                                    create_time: Timestamp::from_seconds(10),
                                    est_release_time: None,
                                    reconciled: batch_id == 0,
                                    undelegation_er: Decimal::from_ratio(102u128, 100u128),
                                    undelegated_stake: Uint128::from(0u128),
                                    unbonding_slashing_ratio: Decimal::zero(),
                                }),
                            })
                            .into(),
                        ),
                        stader::msg::QueryMsg::State {} => SystemResult::Ok(
                            to_json_binary(&stader::msg::QueryStateResponse {
                                state: stader::state::State {
                                    total_staked: Uint128::from(100u128),
                                    exchange_rate: Decimal::from_ratio(102u128, 100u128),
                                    last_reconciled_batch_id: 0,
                                    current_undelegation_batch_id: 11,
                                    last_undelegation_time: Timestamp::from_seconds(10),
                                    last_reinvest_time: Timestamp::from_seconds(10),
                                    validators: vec![],
                                    reconciled_funds_to_withdraw: Uint128::from(100u128),
                                },
                            })
                            .into(),
                        ),
                        _ => err_unsupported_query(msg),
                    };
                }

                err_unsupported_query(msg)
            },

            QueryRequest::Bank(query) => self.bank_querier.query(query),

            QueryRequest::Staking(query) => self.staking_querier.query(query),

            _ => err_unsupported_query(request),
        }
    }

    pub(crate) fn set_bank_balance(&mut self, amount: u128) {
        self.set_bank_balances(&[coin(amount, "utoken")]);
    }
}
