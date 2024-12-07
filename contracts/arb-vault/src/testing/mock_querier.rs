// use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
// use cosmwasm_std::{
//     from_binary, from_slice, to_json_binary, Addr, Coin, Decimal, OwnedDeps, Querier, QuerierResult,
//     QueryRequest, SystemError, SystemResult, Timestamp, Uint128, WasmQuery,
// };
// use prism_protocol::vault::{
//     QueryMsg as PrismQueryMsg, UnbondRequestsResponse, WithdrawableUnbondedResponse,
// };

// use basset::hub::{QueryMsg as AnchorQueryMsg, StateResponse};
// use std::collections::HashMap;
// use std::str::FromStr;
// use std::vec;

// use cw20::{BalanceResponse, Cw20QueryMsg, TokenInfoResponse};
// use terra_cosmwasm::{TaxCapResponse, TaxRateResponse, TerraQuery, TerraQueryWrapper, TerraRoute};

// use crate::lsds::stader::{self, StaderQueries};
// use crate::lsds::steak::{self, QueryMsg as SteakQueryMsg};
// use eris::factory::QueryMsg as FactoryQueryMsg;

// /// mock_dependencies is a drop-in replacement for cosmwasm_std::testing::mock_dependencies.
// pub fn mock_dependencies(
//     contract_balance: &[Coin],
// ) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
//     let custom_querier: WasmMockQuerier =
//         WasmMockQuerier::new(MockQuerier::new(&[(MOCK_CONTRACT_ADDR, contract_balance)]));

//     OwnedDeps {
//         storage: MockStorage::default(),
//         api: MockApi::default(),
//         querier: custom_querier,
//     }
// }

// pub struct WasmMockQuerier {
//     base: MockQuerier<TerraQueryWrapper>,
//     token_querier: TokenQuerier,
//     tax_querier: TaxQuerier,
//     unbonding_amount: Uint128,
//     unbonding_amount_bluna: Option<Uint128>,
//     withdrawable_amount: Uint128,
// }

// #[derive(Clone, Default)]
// pub struct TokenQuerier {
//     // This lets us iterate over all pairs that match the first string
//     balances: HashMap<String, HashMap<String, Uint128>>,
// }

// impl TokenQuerier {
//     pub fn new(balances: &[(&String, &[(&String, &Uint128)])]) -> Self {
//         TokenQuerier {
//             balances: balances_to_map(balances),
//         }
//     }
// }

// pub(crate) fn balances_to_map(
//     balances: &[(&String, &[(&String, &Uint128)])],
// ) -> HashMap<String, HashMap<String, Uint128>> {
//     let mut balances_map: HashMap<String, HashMap<String, Uint128>> = HashMap::new();
//     for (contract_addr, balances) in balances.iter() {
//         let mut contract_balances_map: HashMap<String, Uint128> = HashMap::new();
//         for (addr, balance) in balances.iter() {
//             contract_balances_map.insert(addr.to_string(), **balance);
//         }

//         balances_map.insert(contract_addr.to_string(), contract_balances_map);
//     }
//     balances_map
// }

// #[derive(Clone, Default)]
// pub struct TaxQuerier {
//     rate: Decimal,
//     // This lets us iterate over all pairs that match the first string
//     caps: HashMap<String, Uint128>,
// }

// impl Querier for WasmMockQuerier {
//     fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
//         // MockQuerier doesn't support Custom, so we ignore it completely
//         let request: QueryRequest<TerraQueryWrapper> = match from_slice(bin_request) {
//             Ok(v) => v,
//             Err(e) => {
//                 return SystemResult::Err(SystemError::InvalidRequest {
//                     error: format!("Parsing query request: {}", e),
//                     request: bin_request.into(),
//                 })
//             },
//         };
//         self.handle_query(&request)
//     }
// }

// impl WasmMockQuerier {
//     pub fn handle_query(&self, request: &QueryRequest<TerraQueryWrapper>) -> QuerierResult {
//         match &request {
//             QueryRequest::Custom(TerraQueryWrapper {
//                 route,
//                 query_data,
//             }) => {
//                 if route == &TerraRoute::Treasury {
//                     match query_data {
//                         TerraQuery::TaxRate {} => {
//                             let res = TaxRateResponse {
//                                 rate: self.tax_querier.rate,
//                             };
//                             SystemResult::Ok(to_json_binary(&res).into())
//                         },
//                         TerraQuery::TaxCap {
//                             denom,
//                         } => {
//                             let cap = self.tax_querier.caps.get(denom).copied().unwrap_or_default();
//                             let res = TaxCapResponse {
//                                 cap,
//                             };
//                             SystemResult::Ok(to_json_binary(&res).into())
//                         },
//                         _ => panic!("DO NOT ENTER HERE"),
//                     }
//                 } else {
//                     panic!("DO NOT ENTER HERE")
//                 }
//             },
//             QueryRequest::Wasm(WasmQuery::Smart {
//                 contract_addr,
//                 msg,
//             }) => {
//                 if contract_addr == "prism" {
//                     match from_binary(msg).unwrap() {
//                         PrismQueryMsg::UnbondRequests {
//                             address,
//                             ..
//                         } => SystemResult::Ok(
//                             to_json_binary(&UnbondRequestsResponse {
//                                 address,
//                                 requests: vec![(1u64, self.unbonding_amount)],
//                             })
//                             .into(),
//                         ),
//                         PrismQueryMsg::WithdrawableUnbonded {
//                             ..
//                         } => SystemResult::Ok(
//                             to_json_binary(&WithdrawableUnbondedResponse {
//                                 withdrawable: self.withdrawable_amount,
//                             })
//                             .into(),
//                         ),
//                         PrismQueryMsg::Config {
//                             ..
//                         } => SystemResult::Ok(
//                             to_json_binary(&prism_protocol::vault::StateResponse {
//                                 exchange_rate: Decimal::one(),
//                                 total_bond_amount: Uint128::zero(),
//                                 last_index_modification: 0,
//                                 prev_vault_balance: Uint128::zero(),
//                                 actual_unbonded_amount: Uint128::zero(),
//                                 last_unbonded_time: 0,
//                                 last_processed_batch: 0,
//                             })
//                             .into(),
//                         ),
//                         PrismQueryMsg::AllHistory {
//                             ..
//                         } => SystemResult::Ok(
//                             to_json_binary(&prism_protocol::vault::AllHistoryResponse {
//                                 history: vec![prism_protocol::vault::UnbondHistoryResponse {
//                                     amount: Uint128::zero(),
//                                     batch_id: 0,
//                                     time: 0,
//                                     applied_exchange_rate: Decimal::one(),
//                                     withdraw_rate: Decimal::one(),
//                                     released: false,
//                                 }],
//                             })
//                             .into(),
//                         ),
//                         _ => panic!("DO NOT ENTER HERE"),
//                     }
//                 } else if contract_addr == "steak" {
//                     match from_binary(msg).unwrap() {
//                         SteakQueryMsg::PendingBatch {} => SystemResult::Ok(
//                             to_json_binary(&steak::PendingBatch {
//                                 id: 3,
//                                 usteak_to_burn: Uint128::from(1000u128),
//                                 est_unbond_start_time: 123,
//                             })
//                             .into(),
//                         ),
//                         SteakQueryMsg::PreviousBatch(id) => SystemResult::Ok(
//                             to_json_binary(&steak::Batch {
//                                 id,
//                                 reconciled: id < 2,
//                                 total_shares: Uint128::from(1000u128),
//                                 uluna_unclaimed: Uint128::from(1000u128),
//                                 est_unbond_end_time: 100,
//                             })
//                             .into(),
//                         ),
//                         SteakQueryMsg::UnbondRequestsByUser {
//                             ..
//                         } => SystemResult::Ok(
//                             to_json_binary(&vec![
//                                 steak::UnbondRequestsByUserResponseItem {
//                                     id: 1,
//                                     shares: self.withdrawable_amount,
//                                 },
//                                 steak::UnbondRequestsByUserResponseItem {
//                                     id: 2,
//                                     shares: self.unbonding_amount,
//                                 },
//                             ])
//                             .into(),
//                         ),
//                         SteakQueryMsg::State {} => SystemResult::Ok(
//                             to_json_binary(&steak::StateResponse {
//                                 total_usteak: Uint128::from(1000u128),
//                                 total_uluna: Uint128::from(1000u128),
//                                 exchange_rate: Decimal::one(),
//                                 unlocked_coins: vec![],
//                             })
//                             .into(),
//                         ),
//                         _ => panic!("DO NOT ENTER HERE"),
//                     }
//                 } else if contract_addr == "anchor" {
//                     match from_binary(msg).unwrap() {
//                         AnchorQueryMsg::UnbondRequests {
//                             address,
//                             ..
//                         } => SystemResult::Ok(
//                             to_json_binary(&basset::hub::UnbondRequestsResponse {
//                                 address,
//                                 requests: vec![(
//                                     1u64,
//                                     self.unbonding_amount * Uint128::from(2u128)
//                                         + self.unbonding_amount_bluna.unwrap_or(Uint128::zero()),
//                                     self.unbonding_amount,
//                                 )],
//                             })
//                             .into(),
//                         ),
//                         AnchorQueryMsg::AllHistory {
//                             ..
//                         } => SystemResult::Ok(
//                             to_json_binary(&basset::hub::AllHistoryResponse {
//                                 history: vec![basset::hub::UnbondHistoryResponse {
//                                     batch_id: 1u64,
//                                     time: 1,
//                                     bluna_amount: Uint128::from(1u128),
//                                     bluna_applied_exchange_rate: Decimal::from_ratio(
//                                         101u128, 100u128,
//                                     ),
//                                     bluna_withdraw_rate: Decimal::one(),
//                                     stluna_amount: Uint128::from(1u128),
//                                     stluna_applied_exchange_rate: Decimal::from_ratio(
//                                         103u128, 100u128,
//                                     ),
//                                     stluna_withdraw_rate: Decimal::one(),
//                                     released: false,
//                                     amount: Uint128::from(1u128),
//                                     applied_exchange_rate: Decimal::from_ratio(101u128, 100u128),
//                                     withdraw_rate: Decimal::one(),
//                                 }],
//                             })
//                             .into(),
//                         ),
//                         AnchorQueryMsg::WithdrawableUnbonded {
//                             ..
//                         } => SystemResult::Ok(
//                             to_json_binary(&WithdrawableUnbondedResponse {
//                                 // bluna + stluna + nluna
//                                 withdrawable: self.withdrawable_amount * Uint128::from(3u128),
//                             })
//                             .into(),
//                         ),
//                         AnchorQueryMsg::State {} => SystemResult::Ok(
//                             to_json_binary(&StateResponse {
//                                 bluna_exchange_rate: Decimal::from_ratio(101u128, 100u128),
//                                 stluna_exchange_rate: Decimal::from_ratio(103u128, 100u128),
//                                 total_bond_bluna_amount: Uint128::zero(),
//                                 total_bond_stluna_amount: Uint128::zero(),
//                                 last_index_modification: 0u64,
//                                 prev_hub_balance: Uint128::zero(),
//                                 last_unbonded_time: 0u64,
//                                 last_processed_batch: 0u64,
//                                 total_bond_amount: Uint128::zero(),
//                                 exchange_rate: Decimal::from_ratio(101u128, 100u128),
//                             })
//                             .into(),
//                         ),
//                         _ => panic!("DO NOT ENTER HERE"),
//                     }
//                 } else if contract_addr == "stader" {
//                     match from_binary(msg).unwrap() {
//                         StaderQueries::GetUserUndelegationRecords {
//                             ..
//                         } => SystemResult::Ok(
//                             to_json_binary(&vec![
//                                 stader::UndelegationInfo {
//                                     batch_id: 0u64,
//                                     token_amount: self.withdrawable_amount,
//                                 },
//                                 stader::UndelegationInfo {
//                                     batch_id: 1u64,
//                                     token_amount: self.unbonding_amount,
//                                 },
//                             ])
//                             .into(),
//                         ),
//                         StaderQueries::BatchUndelegation {
//                             batch_id,
//                         } => SystemResult::Ok(
//                             to_json_binary(&stader::QueryBatchUndelegationResponse {
//                                 batch: Some(stader::BatchUndelegationRecord {
//                                     create_time: Timestamp::from_seconds(10),
//                                     est_release_time: None,
//                                     reconciled: batch_id == 0,
//                                     undelegated_stake: Uint128::from(0u128),
//                                     undelegated_tokens: Uint128::from(0u128),
//                                     undelegation_er: Decimal::from_ratio(102u128, 100u128),
//                                     unbonding_slashing_ratio: Decimal::zero(),
//                                 }),
//                             })
//                             .into(),
//                         ),
//                         StaderQueries::State {} => SystemResult::Ok(
//                             to_json_binary(&stader::QueryStateResponse {
//                                 state: stader::StaderState {
//                                     total_staked: Uint128::from(100u128),
//                                     exchange_rate: Decimal::from_ratio(102u128, 100u128),
//                                     last_reconciled_batch_id: 0,
//                                     current_undelegation_batch_id: 11,
//                                     last_undelegation_time: Timestamp::from_seconds(10),
//                                     last_swap_time: Timestamp::from_seconds(10),
//                                     last_reinvest_time: Timestamp::from_seconds(10),
//                                     validators: vec![],
//                                     reconciled_funds_to_withdraw: Uint128::from(100u128),
//                                 },
//                             })
//                             .into(),
//                         ),
//                     }
//                 } else if contract_addr == "factory" {
//                     match from_binary(msg).unwrap() {
//                         FactoryQueryMsg::IsWhitelistedExecutor {
//                             contract_addr,
//                         } => SystemResult::Ok(
//                             to_json_binary(&eris::factory::WhitelistResponse {
//                                 whitelisted: contract_addr == "whitelisted_exec",
//                             })
//                             .into(),
//                         ),
//                         FactoryQueryMsg::IsWhitelistedContract {
//                             contract_addr,
//                         } => SystemResult::Ok(
//                             to_json_binary(&eris::factory::WhitelistResponse {
//                                 whitelisted: contract_addr == "whitelisted",
//                             })
//                             .into(),
//                         ),
//                         FactoryQueryMsg::FeeConfig {} => SystemResult::Ok(
//                             to_json_binary(&eris::factory::FeeConfigResponse {
//                                 fee_address: Addr::unchecked("fee"),
//                                 withdraw_fee: Decimal::from_str("0.001").unwrap(),
//                                 performance_fee: Decimal::from_str("0.1").unwrap(),
//                                 immediate_withdraw_fee: Decimal::from_str("0.02").unwrap(),
//                             })
//                             .into(),
//                         ),
//                         FactoryQueryMsg::Config {} => SystemResult::Ok(
//                             to_json_binary(&eris::factory::FactoryConfigResponse {
//                                 owner: Addr::unchecked("owner"),
//                                 pool_configs: vec![],
//                                 token_code_id: 123u64,
//                                 fee_address: Addr::unchecked("fee"),
//                                 withdraw_fee: Decimal::from_str("0.001").unwrap(),
//                                 performance_fee: Decimal::from_str("0.1").unwrap(),
//                                 immediate_withdraw_fee: Decimal::from_str("0.02").unwrap(),
//                                 pools: vec![],
//                             })
//                             .into(),
//                         ),
//                         _ => panic!("DO NOT ENTER HERE"),
//                     }
//                 } else {
//                     match from_binary(msg).unwrap() {
//                         Cw20QueryMsg::TokenInfo {} => {
//                             let balances: &HashMap<String, Uint128> =
//                                 match self.token_querier.balances.get(contract_addr) {
//                                     Some(balances) => balances,
//                                     None => {
//                                         return SystemResult::Err(SystemError::Unknown {});
//                                     },
//                                 };

//                             let mut total_supply = Uint128::zero();

//                             for balance in balances {
//                                 total_supply += *balance.1;
//                             }

//                             SystemResult::Ok(
//                                 to_json_binary(&TokenInfoResponse {
//                                     name: "erisLUNA-LP".to_string(),
//                                     symbol: "erisLUNA".to_string(),
//                                     decimals: 6,
//                                     total_supply,
//                                 })
//                                 .into(),
//                             )
//                         },
//                         Cw20QueryMsg::Balance {
//                             address,
//                         } => {
//                             let balances: &HashMap<String, Uint128> =
//                                 match self.token_querier.balances.get(contract_addr) {
//                                     Some(balances) => balances,
//                                     None => {
//                                         return SystemResult::Err(SystemError::Unknown {});
//                                     },
//                                 };

//                             let balance = match balances.get(&address) {
//                                 Some(v) => v,
//                                 None => {
//                                     return SystemResult::Err(SystemError::Unknown {});
//                                 },
//                             };

//                             SystemResult::Ok(
//                                 to_json_binary(&BalanceResponse {
//                                     balance: *balance,
//                                 })
//                                 .into(),
//                             )
//                         },
//                         _ => panic!("DO NOT ENTER HERE"),
//                     }
//                 }
//             },
//             _ => self.base.handle_query(request),
//         }
//     }
// }

// impl WasmMockQuerier {
//     pub fn new(base: MockQuerier<TerraQueryWrapper>) -> Self {
//         WasmMockQuerier {
//             base,
//             token_querier: TokenQuerier::default(),
//             tax_querier: TaxQuerier::default(),
//             unbonding_amount: Uint128::zero(),
//             unbonding_amount_bluna: None,
//             withdrawable_amount: Uint128::zero(),
//         }
//     }

//     // Configure the mint whitelist mock querier
//     pub fn with_token_balances(&mut self, balances: &[(&String, &[(&String, &Uint128)])]) {
//         self.token_querier = TokenQuerier::new(balances);
//     }

//     pub fn with_balance(&mut self, balances: &[(&String, &[Coin])]) {
//         for (addr, balance) in balances {
//             self.base.update_balance(addr.to_string(), balance.to_vec());
//         }
//     }

//     pub fn with_unbonding(&mut self, amount: Uint128) {
//         self.unbonding_amount = amount;
//     }
//     pub fn with_unbonding_bluna(&mut self, amount: Uint128) {
//         self.unbonding_amount_bluna = Some(amount);
//     }
//     pub fn with_withdrawable(&mut self, amount: Uint128) {
//         self.withdrawable_amount = amount;
//     }
// }
