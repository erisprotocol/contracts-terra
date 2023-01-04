use cosmwasm_std::{QuerierResult, SystemError, SystemResult};

pub(super) fn err_unsupported_query<T: std::fmt::Debug>(request: T) -> QuerierResult {
    SystemResult::Err(SystemError::InvalidRequest {
        error: format!("[mock] unsupported query: {:?}", request),
        request: Default::default(),
    })
}

// pub(super) fn mock_dependencies() -> OwnedDeps<MockStorage, MockApi, CustomQuerier> {
//     OwnedDeps {
//         storage: MockStorage::default(),
//         api: MockApi::default(),
//         querier: CustomQuerier::default(),
//         custom_query_type: std::marker::PhantomData::default(),
//     }
// }

// pub(super) fn mock_env_at_timestamp(timestamp: u64) -> Env {
//     Env {
//         block: BlockInfo {
//             height: 12_345,
//             time: Timestamp::from_seconds(timestamp),
//             chain_id: "cosmos-testnet-14002".to_string(),
//         },
//         contract: ContractInfo {
//             address: Addr::unchecked(MOCK_CONTRACT_ADDR),
//         },
//         transaction: None,
//     }
// }

// pub(super) fn query_helper<T: DeserializeOwned>(deps: Deps, msg: QueryMsg) -> T {
//     from_binary(&query(deps, mock_env(), msg).unwrap()).unwrap()
// }

// pub(super) fn query_helper_env<T: DeserializeOwned>(
//     deps: Deps,
//     msg: QueryMsg,
//     timestamp: u64,
// ) -> T {
//     from_binary(&query(deps, mock_env_at_timestamp(timestamp), msg).unwrap()).unwrap()
// }
