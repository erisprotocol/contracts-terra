use std::str::FromStr;

use cosmwasm_std::testing::{mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, to_binary, Addr, BlockInfo, ContractInfo, Decimal, Deps, DepsMut, Env, Event,
    OwnedDeps, QuerierResult, Reply, ReplyOn, SubMsg, SubMsgResponse, SystemError, SystemResult,
    Timestamp, WasmMsg,
};
use cw20::MinterResponse;
use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;
use serde::de::DeserializeOwned;

use eris::arb_vault::{InstantiateMsg, LsdConfig, QueryMsg};

use crate::constants::INSTANTIATE_TOKEN_REPLY_ID;
use crate::contract::{instantiate, query, reply};

use super::custom_querier::CustomQuerier;

pub(super) fn err_unsupported_query<T: std::fmt::Debug>(request: T) -> QuerierResult {
    SystemResult::Err(SystemError::InvalidRequest {
        error: format!("[mock] unsupported query: {:?}", request),
        request: Default::default(),
    })
}

pub(super) fn mock_dependencies() -> OwnedDeps<MockStorage, MockApi, CustomQuerier> {
    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: CustomQuerier::default(),
        custom_query_type: std::marker::PhantomData::default(),
    }
}

pub(super) fn _mock_env_at_timestamp(timestamp: u64) -> Env {
    Env {
        block: BlockInfo {
            height: 12_345,
            time: Timestamp::from_seconds(timestamp),
            chain_id: "cosmos-testnet-14002".to_string(),
        },
        contract: ContractInfo {
            address: Addr::unchecked(MOCK_CONTRACT_ADDR),
        },
        transaction: None,
    }
}

pub(super) fn _query_helper<T: DeserializeOwned>(deps: Deps, msg: QueryMsg) -> T {
    from_binary(&query(deps, mock_env(), msg).unwrap()).unwrap()
}

pub(super) fn _query_helper_env<T: DeserializeOwned>(
    deps: Deps,
    msg: QueryMsg,
    timestamp: u64,
) -> T {
    from_binary(&query(deps, _mock_env_at_timestamp(timestamp), msg).unwrap()).unwrap()
}

fn store_liquidity_token(deps: DepsMut, _msg_id: u64, contract_addr: String) {
    let event = Event::new("instantiate")
        .add_attribute("creator", MOCK_CONTRACT_ADDR)
        .add_attribute("admin", "admin")
        .add_attribute("code_id", "69420")
        .add_attribute("_contract_address", contract_addr);

    let _res = reply(
        deps,
        mock_env(),
        Reply {
            id: INSTANTIATE_TOKEN_REPLY_ID,
            result: cosmwasm_std::SubMsgResult::Ok(SubMsgResponse {
                events: vec![event],
                data: None,
            }),
        },
    )
    .unwrap();
}

pub fn create_default_lsd_configs() -> Vec<LsdConfig<String>> {
    vec![
        LsdConfig {
            disabled: false,
            name: "eris".into(),
            lsd_type: eris::arb_vault::LsdType::Eris {
                addr: "eris".into(),
                cw20: "eriscw".into(),
            },
        },
        LsdConfig {
            disabled: false,
            name: "backbone".into(),
            lsd_type: eris::arb_vault::LsdType::Backbone {
                addr: "backbone".into(),
                cw20: "backbonecw".into(),
            },
        },
        LsdConfig {
            disabled: false,
            name: "stader".into(),
            lsd_type: eris::arb_vault::LsdType::Stader {
                addr: "stader".into(),
                cw20: "stadercw".into(),
            },
        },
        LsdConfig {
            disabled: false,
            name: "prism".into(),
            lsd_type: eris::arb_vault::LsdType::Prism {
                addr: "prism".into(),
                cw20: "prismcw".into(),
            },
        },
    ]
}

pub fn mock_env() -> Env {
    Env {
        block: BlockInfo {
            height: 12_345,
            time: Timestamp::from_seconds(1),
            chain_id: "cosmos-testnet-14002".to_string(),
        },
        contract: ContractInfo {
            address: Addr::unchecked(MOCK_CONTRACT_ADDR),
        },
        transaction: None,
    }
}

// fn mock_env_51() -> Env {
//     Env {
//         block: BlockInfo {
//             height: 12_345,
//             time: Timestamp::from_seconds(51),
//             chain_id: "cosmos-testnet-14002".to_string(),
//         },
//         contract: ContractInfo {
//             address: Addr::unchecked(MOCK_CONTRACT_ADDR),
//         },
//         transaction: None,
//     }
// }
// fn mock_env_200() -> Env {
//     Env {
//         block: BlockInfo {
//             height: 12_345,
//             time: Timestamp::from_seconds(200),
//             chain_id: "cosmos-testnet-14002".to_string(),
//         },
//         contract: ContractInfo {
//             address: Addr::unchecked(MOCK_CONTRACT_ADDR),
//         },
//         transaction: None,
//     }
// }
// fn mock_env_130() -> Env {
//     Env {
//         block: BlockInfo {
//             height: 12_345,
//             time: Timestamp::from_seconds(130),
//             chain_id: "cosmos-testnet-14002".to_string(),
//         },
//         contract: ContractInfo {
//             address: Addr::unchecked(MOCK_CONTRACT_ADDR),
//         },
//         transaction: None,
//     }
// }

// fn create_init_params() -> Option<Binary> {
//     Some(to_binary(&create_default_lsd_configs()).unwrap())
// }

pub fn create_default_init() -> InstantiateMsg {
    InstantiateMsg {
        cw20_code_id: 10u64,
        name: "arbname".into(),
        symbol: "arbsymbol".into(),
        decimals: 6,
        owner: "owner".into(),
        utoken: "utoken".into(),
        utilization_method: eris::arb_vault::UtilizationMethod::Steps(vec![
            (
                // 1% = 50% of pool
                Decimal::from_ratio(10u128, 1000u128),
                Decimal::from_ratio(50u128, 100u128),
            ),
            (
                // 1% = 50% of pool
                Decimal::from_ratio(15u128, 1000u128),
                Decimal::from_ratio(70u128, 100u128),
            ),
            (
                // 1% = 50% of pool
                Decimal::from_ratio(20u128, 1000u128),
                Decimal::from_ratio(90u128, 100u128),
            ),
            (
                // 1% = 50% of pool
                Decimal::from_ratio(25u128, 1000u128),
                Decimal::from_ratio(100u128, 100u128),
            ),
        ]),
        unbond_time_s: 100,
        lsds: create_default_lsd_configs(),
        fee_config: eris::arb_vault::FeeConfig {
            protocol_fee_contract: "fee".into(),
            protocol_performance_fee: Decimal::from_str("0.01").unwrap(),
            protocol_withdraw_fee: Decimal::from_str("0.02").unwrap(),
            immediate_withdraw_fee: Decimal::from_str("0.05").unwrap(),
        },
        whitelist: vec!["whitelisted_exec".to_string()],
    }
}

pub(super) fn setup_test() -> OwnedDeps<MockStorage, MockApi, CustomQuerier> {
    let mut deps = mock_dependencies();
    let msg = create_default_init();
    let owner = "owner";
    let owner_info = mock_info(owner, &[]);
    let res = instantiate(deps.as_mut(), mock_env(), owner_info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg {
            msg: WasmMsg::Instantiate {
                code_id: 10u64,
                msg: to_binary(&Cw20InstantiateMsg {
                    name: "arbname".to_string(),
                    symbol: "arbsymbol".to_string(),
                    decimals: 6,
                    initial_balances: vec![],
                    mint: Some(MinterResponse {
                        minter: String::from(MOCK_CONTRACT_ADDR),
                        cap: None,
                    }),
                    marketing: None
                })
                .unwrap(),
                funds: vec![],
                admin: Some("owner".into()),
                label: String::from("Eris Arb Vault LP Token"),
            }
            .into(),
            id: 1,
            gas_limit: None,
            reply_on: ReplyOn::Success
        },]
    );

    store_liquidity_token(deps.as_mut(), 1, "lptoken".to_string());
    deps.querier.set_cw20_balance("eriscw", MOCK_CONTRACT_ADDR, 0u128);
    deps.querier.set_cw20_balance("backbonecw", MOCK_CONTRACT_ADDR, 0u128);
    deps.querier.set_cw20_balance("stadercw", MOCK_CONTRACT_ADDR, 0u128);
    deps.querier.set_cw20_balance("prismcw", MOCK_CONTRACT_ADDR, 0u128);

    deps
}
