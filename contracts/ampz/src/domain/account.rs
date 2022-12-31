// use cosmwasm_std::{
//     to_vec, Binary, ContractResult, DepsMut, QueryRequest, StdError, StdResult, SystemResult,
// };
// use protobuf::Message;

// use crate::protos::proto::{
//     BaseAccount, PeriodicVestingAccount, QueryAccountRequest, QueryAccountResponse,
// };

// pub fn read_account(deps: &DepsMut, user: impl Into<String>) -> StdResult<()> {
//     let request: QueryRequest<()> = cosmwasm_std::QueryRequest::Stargate {
//         path: "cosmos.auth.v1beta1.Query/Account".to_string(),
//         data: Binary::from(
//             QueryAccountRequest {
//                 address: user.into(),
//                 special_fields: Default::default(),
//             }
//             .to_bytes()?,
//         ),
//     };

//     let raw = to_vec(&request).map_err(|serialize_err| {
//         StdError::generic_err(format!("Serializing QueryRequest: {}", serialize_err))
//     })?;

//     let response = match deps.querier.raw_query(&raw) {
//         SystemResult::Err(system_err) => {
//             Err(StdError::generic_err(format!("Querier system error: {}", system_err)))
//         },
//         SystemResult::Ok(ContractResult::Err(contract_err)) => {
//             Err(StdError::generic_err(format!("Querier contract error: {}", contract_err)))
//         },
//         SystemResult::Ok(ContractResult::Ok(value)) => {
//             Ok(QueryAccountResponse::parse_from_bytes(&value).map_err(|serialize_err| {
//                 StdError::generic_err(format!(
//                     "Deserializing QueryAccountResponse: {}",
//                     serialize_err
//                 ))
//             })?)
//         },
//     }?;

//     if response.account.type_url == "/cosmos.vesting.v1beta1.PeriodicVestingAccount" {
//         let periodic = PeriodicVestingAccount::parse_from_bytes(&response.account.value).map_err(
//             |serialize_err| {
//                 StdError::generic_err(format!(
//                     "Deserializing PeriodicVestingAccount: {}",
//                     serialize_err
//                 ))
//             },
//         )?;

//         return Err(StdError::generic_err(format!(
//             "periods: {0} start: {1}, end {2}",
//             periodic.vesting_periods.len(),
//             periodic.start_time,
//             periodic.base_vesting_account.end_time,
//         )));
//     } else if response.account.type_url == "/cosmos.auth.v1beta1.BaseAccount" {
//         let base =
//             BaseAccount::parse_from_bytes(&response.account.value).map_err(|serialize_err| {
//                 StdError::generic_err(format!("Deserializing BaseAccount: {}", serialize_err))
//             })?;
//         return Err(StdError::generic_err(format!(
//             "base account_number {0} sequence {1}",
//             base.account_number, base.sequence
//         )));
//     } else {
//         return Err(StdError::generic_err(format!(
//             "not supported account type: {}",
//             response.account.type_url
//         )));
//     }
// }
