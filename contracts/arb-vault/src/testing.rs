// use std::str::FromStr;

// use crate::lsds::stader::StaderExecute;

// use crate::lsds::steak::{self, ReceiveMsg as SteakReceiveMsg};
// use crate::{
//     contract::{execute, instantiate, reply},
//     error::ContractError,
// };

// use crate::query::{
//     query_balance_details, query_balances, query_config, query_takeable, query_unbond_requests,
//     query_user_info, query_withdrawable_unbonded,
// };

// use crate::mock_querier::{mock_dependencies, WasmMockQuerier};
// use crate::response::MsgInstantiateContractResponse;

// use eris::asset::{Asset, AssetInfo};
// use eris::pool::{
//     BalancesResponse, CalcMethod, ClaimBalance, Config, ConfigResponse, Cw20HookMsg, ExecuteMsg,
//     ExecuteSubMsg, InstantiateMsg, LunaPoolParams, PartialPoolParams, PoolParams, TakeableResponse,
//     UnbondItem, UnbondResponse, WithdrawableResponse,
// };
// use eris::token::InstantiateMsg as TokenInstantiateMsg;
// use eris::DecimalCheckedOps;

// use cosmwasm_std::testing::{mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
// use cosmwasm_std::{
//     attr, from_binary, to_binary, Addr, BankMsg, Binary, BlockInfo, Coin, ContractInfo,
//     ContractResult, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, OwnedDeps, Reply, ReplyOn,
//     Response, StdError, SubMsg, SubMsgExecutionResponse, Timestamp, Uint128, WasmMsg,
// };

// use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, MinterResponse};

// use protobuf::Message;
// use schemars::JsonSchema;
// use serde::{Deserialize, Serialize};

// macro_rules! assert_delta {
//     ($x:expr, $y:expr, $d:expr) => {
//         if ($x > $y + $d || $x < $y - $d) {
//             panic!();
//         }
//     };
// }

// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// #[serde(rename_all = "snake_case")]
// struct Empty {}

// fn store_liquidity_token(deps: DepsMut, msg_id: u64, contract_addr: String) {
//     let data = MsgInstantiateContractResponse {
//         contract_address: contract_addr,
//         data: vec![],
//         unknown_fields: Default::default(),
//         cached_size: Default::default(),
//     }
//     .write_to_bytes()
//     .unwrap();

//     let reply_msg = Reply {
//         id: msg_id,
//         result: ContractResult::Ok(SubMsgExecutionResponse {
//             events: vec![],
//             data: Some(data.into()),
//         }),
//     };

//     let _res = reply(deps, mock_env(), reply_msg).unwrap();
// }

// fn create_default_pool_params() -> LunaPoolParams {
//     LunaPoolParams {
//         cluna_active: true,
//         cluna_addr: "prism".to_string(),
//         cluna_cw20: Addr::unchecked("cluna"),
//         bluna_active: true,
//         bluna_addr: "anchor".to_string(),
//         bluna_cw20: Addr::unchecked("bluna"),
//         stluna_active: true,
//         stluna_cw20: Addr::unchecked("stluna"),
//         nluna_active: true,
//         nluna_addr: "nexus".to_string(),
//         nluna_cw20: Addr::unchecked("nluna"),

//         lunax_active: true,
//         lunax_addr: "stader".to_string(),
//         lunax_cw20: Addr::unchecked("lunax"),

//         steak_active: true,
//         steak_addr: "steak".to_string(),
//         steak_cw20: Addr::unchecked("steak_cw"),
//     }
// }

// fn mock_env() -> Env {
//     Env {
//         block: BlockInfo {
//             height: 12_345,
//             time: Timestamp::from_seconds(1),
//             chain_id: "cosmos-testnet-14002".to_string(),
//         },
//         contract: ContractInfo {
//             address: Addr::unchecked(MOCK_CONTRACT_ADDR),
//         },
//     }
// }

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
//     }
// }

// fn create_init_params() -> Option<Binary> {
//     Some(to_binary(&create_default_pool_params()).unwrap())
// }

// fn create_default_init() -> InstantiateMsg {
//     InstantiateMsg {
//         factory_addr: String::from("factory"),
//         pool_params: PoolParams {
//             utilization_method: eris::pool::CalcMethod::Steps(vec![
//                 (
//                     // 1% = 50% of pool
//                     Decimal::from_ratio(10u128, 1000u128),
//                     Decimal::from_ratio(50u128, 100u128),
//                 ),
//                 (
//                     // 1% = 50% of pool
//                     Decimal::from_ratio(15u128, 1000u128),
//                     Decimal::from_ratio(70u128, 100u128),
//                 ),
//                 (
//                     // 1% = 50% of pool
//                     Decimal::from_ratio(20u128, 1000u128),
//                     Decimal::from_ratio(90u128, 100u128),
//                 ),
//                 (
//                     // 1% = 50% of pool
//                     Decimal::from_ratio(25u128, 1000u128),
//                     Decimal::from_ratio(100u128, 100u128),
//                 ),
//             ]),
//             unbond_time_s: 100,
//         },
//         token_code_id: 10u64,
//         init_params: create_init_params(),
//     }
// }

// fn setup() -> (OwnedDeps<MockStorage, MockApi, WasmMockQuerier>, Env, MessageInfo) {
//     let mut deps = mock_dependencies(&[]);
//     let msg = create_default_init();
//     let owner = "owner";
//     let env = mock_env();
//     let owner_info = mock_info(owner, &[]);
//     let _res = instantiate(deps.as_mut(), env.clone(), owner_info.clone(), msg).unwrap();
//     store_liquidity_token(deps.as_mut(), 1, "lptoken".to_string());

//     (deps, env, owner_info)
// }

// #[test]
// fn proper_initialization() {
//     let mut deps = mock_dependencies(&[]);
//     let msg = create_default_init();

//     let owner = "owner";
//     // We can just call .unwrap() to assert this was a success
//     let env = mock_env();
//     let owner_info = mock_info(owner, &[]);
//     let res = instantiate(deps.as_mut(), env, owner_info, msg).unwrap();

//     assert_eq!(
//         res.messages,
//         vec![SubMsg {
//             msg: WasmMsg::Instantiate {
//                 code_id: 10u64,
//                 msg: to_binary(&TokenInstantiateMsg {
//                     name: "erisLUNA-LP".to_string(),
//                     symbol: "erisLUNA".to_string(),
//                     decimals: 6,
//                     initial_balances: vec![],
//                     mint: Some(MinterResponse {
//                         minter: String::from(MOCK_CONTRACT_ADDR),
//                         cap: None,
//                     }),
//                 })
//                 .unwrap(),
//                 funds: vec![],
//                 admin: None,
//                 label: String::from("Eris LP token"),
//             }
//             .into(),
//             id: 1,
//             gas_limit: None,
//             reply_on: ReplyOn::Success
//         },]
//     );

//     // Store liquidity token
//     store_liquidity_token(deps.as_mut(), 1, "lptoken".to_string());

//     // It worked, let's query the state
//     let config: ConfigResponse = query_config(deps.as_ref()).unwrap();

//     assert_eq!(
//         config,
//         ConfigResponse {
//             config: Config {
//                 pool_params: PoolParams {
//                     utilization_method: CalcMethod::Steps(vec![
//                         (
//                             // 1% = 50% of pool
//                             Decimal::from_ratio(10u128, 1000u128),
//                             Decimal::from_ratio(50u128, 100u128),
//                         ),
//                         (
//                             // 1% = 50% of pool
//                             Decimal::from_ratio(15u128, 1000u128),
//                             Decimal::from_ratio(70u128, 100u128),
//                         ),
//                         (
//                             // 1% = 50% of pool
//                             Decimal::from_ratio(20u128, 1000u128),
//                             Decimal::from_ratio(90u128, 100u128),
//                         ),
//                         (
//                             // 1% = 50% of pool
//                             Decimal::from_ratio(25u128, 1000u128),
//                             Decimal::from_ratio(100u128, 100u128),
//                         ),
//                     ]),
//                     unbond_time_s: 100
//                 },
//                 liquidity_token: Addr::unchecked("lptoken"),
//                 pool_type: "LUNA".to_string(),
//                 params: create_default_pool_params(),
//                 factory_addr: Addr::unchecked("factory")
//             }
//         }
//     );
// }

// #[test]
// fn update_config() {
//     let (mut deps, env, owner_info) = setup();

//     let user_info = mock_info(
//         "user001",
//         &[Coin {
//             denom: "notsupported".to_string(),
//             amount: Uint128::from(100_000000u128),
//         }],
//     );
//     let upd_msg = ExecuteMsg::UpdateConfig {
//         pool_params: Some(PartialPoolParams {
//             unbond_time_s: Some(10u64),
//             utilization_method: None,
//         }),
//         luna_params: None,
//     };

//     let res =
//         execute(deps.as_mut(), env.clone(), user_info, upd_msg.clone()).expect_err("expects error");
//     assert_eq!(res, ContractError::Unauthorized {});

//     let _res =
//         execute(deps.as_mut(), env.clone(), owner_info.clone(), upd_msg).expect("expects response");

//     let config = query_config(deps.as_ref()).expect("expects response");

//     assert_eq!(
//         config,
//         ConfigResponse {
//             config: Config {
//                 pool_params: PoolParams {
//                     utilization_method: CalcMethod::Steps(vec![
//                         (
//                             // 1% = 50% of pool
//                             Decimal::from_ratio(10u128, 1000u128),
//                             Decimal::from_ratio(50u128, 100u128),
//                         ),
//                         (
//                             // 1% = 50% of pool
//                             Decimal::from_ratio(15u128, 1000u128),
//                             Decimal::from_ratio(70u128, 100u128),
//                         ),
//                         (
//                             // 1% = 50% of pool
//                             Decimal::from_ratio(20u128, 1000u128),
//                             Decimal::from_ratio(90u128, 100u128),
//                         ),
//                         (
//                             // 1% = 50% of pool
//                             Decimal::from_ratio(25u128, 1000u128),
//                             Decimal::from_ratio(100u128, 100u128),
//                         ),
//                     ]),
//                     unbond_time_s: 10
//                 },
//                 liquidity_token: Addr::unchecked("lptoken"),
//                 pool_type: "LUNA".to_string(),
//                 params: create_default_pool_params(),
//                 factory_addr: Addr::unchecked("factory")
//             }
//         }
//     );

//     let upd_msg = ExecuteMsg::UpdateConfig {
//         pool_params: Some(PartialPoolParams {
//             unbond_time_s: None,
//             utilization_method: Some(CalcMethod::Steps(vec![])),
//         }),
//         luna_params: None,
//     };

//     let _res = execute(deps.as_mut(), env, owner_info, upd_msg).expect("expects response");

//     let config = query_config(deps.as_ref()).expect("expects response");

//     assert_eq!(
//         config,
//         ConfigResponse {
//             config: Config {
//                 pool_params: PoolParams {
//                     utilization_method: CalcMethod::Steps(vec![]),
//                     unbond_time_s: 10
//                 },
//                 liquidity_token: Addr::unchecked("lptoken"),
//                 pool_type: "LUNA".to_string(),
//                 params: create_default_pool_params(),
//                 factory_addr: Addr::unchecked("factory")
//             }
//         }
//     );
// }

// #[test]
// fn provide_liquidity_wrong_token() {
//     let (mut deps, env, _owner_info) = setup();

//     let user_info = mock_info(
//         "user001",
//         &[Coin {
//             denom: "notsupported".to_string(),
//             amount: Uint128::from(100_000000u128),
//         }],
//     );

//     let provide_msg = ExecuteMsg::ProvideLiquidity {
//         asset: Asset {
//             info: AssetInfo::NativeToken {
//                 denom: "notsupported".to_string(),
//             },
//             amount: Uint128::from(100_000000u128),
//         },
//         receiver: None,
//     };

//     let res = execute(deps.as_mut(), env, user_info, provide_msg);

//     assert_eq!(res, Err(ContractError::AssetMismatch {}))
// }

// #[test]
// fn provide_liquidity_wrong_amount() {
//     let (mut deps, env, _owner_info) = setup();

//     let user_info = mock_info(
//         "user001",
//         &[Coin {
//             denom: "uluna".to_string(),
//             amount: Uint128::from(123_000000u128),
//         }],
//     );

//     let provide_msg = ExecuteMsg::ProvideLiquidity {
//         asset: Asset {
//             info: AssetInfo::NativeToken {
//                 denom: "uluna".to_string(),
//             },
//             amount: Uint128::from(100_000000u128),
//         },
//         receiver: None,
//     };

//     let res = execute(deps.as_mut(), env, user_info, provide_msg);

//     res.expect_err("Expects error");
// }

// #[test]
// fn provide_liquidity_zero_throws() {
//     let (mut deps, env, _owner_info) = setup();

//     let user_info = mock_info(
//         "user001",
//         &[Coin {
//             denom: "uluna".to_string(),
//             amount: Uint128::from(0u128),
//         }],
//     );

//     let provide_msg = ExecuteMsg::ProvideLiquidity {
//         asset: Asset {
//             info: AssetInfo::NativeToken {
//                 denom: "uluna".to_string(),
//             },
//             amount: Uint128::from(0u128),
//         },
//         receiver: None,
//     };

//     let res = execute(deps.as_mut(), env, user_info, provide_msg);

//     res.expect_err("Expects error");
// }

// fn _provide_liquidity(
// ) -> (OwnedDeps<MockStorage, MockApi, WasmMockQuerier>, Env, MessageInfo, Response) {
//     let (mut deps, env, owner_info) = setup();

//     deps.querier.with_balance(&[(
//         &String::from(MOCK_CONTRACT_ADDR),
//         &[Coin {
//             denom: "uluna".to_string(),
//             amount: Uint128::new(100_000000u128 /* user deposit must be pre-applied */),
//         }],
//     )]);

//     deps.querier.with_token_balances(&[(
//         &String::from("lptoken"),
//         &[(&String::from(MOCK_CONTRACT_ADDR), &Uint128::new(0))],
//     )]);

//     let user_info = mock_info(
//         "user001",
//         &[Coin {
//             denom: "uluna".to_string(),
//             amount: Uint128::from(100_000000u128),
//         }],
//     );

//     let provide_msg = ExecuteMsg::ProvideLiquidity {
//         asset: Asset {
//             info: AssetInfo::NativeToken {
//                 denom: "uluna".to_string(),
//             },
//             amount: Uint128::from(100_000000u128),
//         },
//         receiver: None,
//     };

//     let res =
//         execute(deps.as_mut(), env.clone(), user_info, provide_msg).expect("should have response");

//     deps.querier.with_token_balances(&[(
//         &String::from("lptoken"),
//         &[(&String::from(MOCK_CONTRACT_ADDR), &Uint128::new(100_000000u128))],
//     )]);

//     (deps, env, owner_info, res)
// }

// #[test]
// fn provide_liquidity_success() {
//     let (_deps, _env, _owner_info, res) = _provide_liquidity();

//     let total_asset = res.attributes.get(3).expect("should have value");
//     let share = res.attributes.get(4).expect("should have value");

//     assert_eq!(total_asset, attr("total_asset", Uint128::from(100_000000u128).to_string()));
//     assert_eq!(share, attr("share", Uint128::from(100_000000u128).to_string()));
// }

// fn _provide_liquidity_again(
// ) -> (OwnedDeps<MockStorage, MockApi, WasmMockQuerier>, Env, MessageInfo, Response) {
//     let (mut deps, env, owner_info, _res) = _provide_liquidity();

//     deps.querier.with_balance(&[(
//         &String::from(MOCK_CONTRACT_ADDR),
//         &[Coin {
//             denom: "uluna".to_string(),
//             amount: Uint128::new(
//                 100_000000u128 + 120_000000u128, /* user deposit must be pre-applied */
//             ),
//         }],
//     )]);

//     let user_info = mock_info(
//         "user001",
//         &[Coin {
//             denom: "uluna".to_string(),
//             amount: Uint128::from(120_000000u128),
//         }],
//     );

//     let provide_msg = ExecuteMsg::ProvideLiquidity {
//         asset: Asset {
//             info: AssetInfo::NativeToken {
//                 denom: "uluna".to_string(),
//             },
//             amount: Uint128::from(120_000000u128),
//         },
//         receiver: None,
//     };

//     let res =
//         execute(deps.as_mut(), env.clone(), user_info, provide_msg).expect("should have response");

//     deps.querier.with_token_balances(&[(
//         &String::from("lptoken"),
//         &[(&String::from(MOCK_CONTRACT_ADDR), &Uint128::new(220_000000u128))],
//     )]);

//     (deps, env, owner_info, res)
// }

// #[test]
// fn provide_liquidity_again_success() {
//     let (_deps, _env, _owner_info, res) = _provide_liquidity_again();

//     let total_asset = res.attributes.get(3).expect("should have value");
//     let share = res.attributes.get(4).expect("should have value");

//     assert_eq!(total_asset, attr("total_asset", Uint128::from(220_000000u128).to_string()));
//     assert_eq!(share, attr("share", Uint128::from(120_000000u128).to_string()));
// }

// #[test]
// fn query_share_check() {
//     let (mut deps, env, _owner_info, _res) = _provide_liquidity_again();

//     let response =
//         query_user_info(deps.as_ref(), env.clone(), Uint128::from(100_000000u128)).unwrap();
//     assert_eq!(response.received_asset, Uint128::from(100_000000u128));

//     // arbs executed and created 2 luna
//     deps.querier.with_balance(&[(
//         &String::from(MOCK_CONTRACT_ADDR),
//         &[Coin {
//             denom: "uluna".to_string(),
//             amount: Uint128::new(
//                 /* through arbs, 2 more luna were generated */
//                 100_000000u128 + 120_000000u128 + 2_000000u128,
//             ),
//         }],
//     )]);

//     let response =
//         query_user_info(deps.as_ref(), env.clone(), Uint128::from(100_000000u128)).unwrap();
//     assert_eq!(
//         response.received_asset,
//         Uint128::from(100_000000u128).multiply_ratio(
//             // pool has 2 more asset
//             100_000000u128 + 120_000000u128 + 2_000000u128,
//             // lp token still at 220
//             100_000000u128 + 120_000000u128
//         )
//     );

//     /* through arbs, 3 more luna are currently unbonding were generated */
//     deps.querier.with_unbonding(Uint128::from(3_000000u128));

//     let response =
//         query_user_info(deps.as_ref(), env.clone(), Uint128::from(100_000000u128)).unwrap();

//     let stader_unbonding = Decimal::from_ratio(102u128, 100u128) * Uint128::from(3_000000u128);
//     let anchor_unbonding =
//         Decimal::from_ratio(101u128, 100u128) * Uint128::from(3_000000u128) * Uint128::from(2u128);

//     let lido_unbonding = Decimal::from_ratio(103u128, 100u128) * Uint128::from(3_000000u128);

//     let steak_unbonding = Uint128::from(3_000000u128);

//     assert_eq!(
//         response.received_asset,
//         Uint128::from(100_000000u128).multiply_ratio(
//             // pool has 2 more asset and 3 unbonding
//             100_000000u128
//                 + 120_000000u128
//                 + 2_000000u128
//                 + 3_000000u128
//                 + stader_unbonding.u128()
//                 + anchor_unbonding.u128()
//                 + lido_unbonding.u128()
//                 + steak_unbonding.u128(),
//             // lp token still at 220
//             100_000000u128 + 120_000000u128
//         )
//     );

//     /* through arbs, 4 more luna can currently be claimed */
//     deps.querier.with_withdrawable(Uint128::from(4_000000u128));

//     let stader_withdrawing = Decimal::from_ratio(102u128, 100u128) * Uint128::from(4_000000u128);
//     let steak_withdrawing = Uint128::from(4_000000u128);

//     let response = query_user_info(deps.as_ref(), env, Uint128::from(100_000000u128)).unwrap();
//     assert_eq!(
//         response.received_asset,
//         Uint128::from(100_000000u128).multiply_ratio(
//             // pool has 2 more asset and 3 unbonding and 4 claimable from liquid staking provider
//             100_000000u128
//                 + 120_000000u128
//                 + 2_000000u128
//                 + 3_000000u128
//                 + stader_unbonding.u128()
//                 + anchor_unbonding.u128()
//                 + lido_unbonding.u128()
//                 +steak_unbonding.u128()
//                 + stader_withdrawing.u128()
//                 + 4_000000u128 // cluna
//                 + 12_000000u128 // bluna + stluna + nluna
//                 + steak_withdrawing.u128(),
//             // lp token still at 220
//             100_000000u128 + 120_000000u128
//         )
//     );
// }

// #[test]
// fn throws_if_provided_profit_not_found() {
//     let (mut deps, env, _owner_info) = setup();

//     let whitelist_info = mock_info("whitelisted_exec", &[]);

//     let exec_msg = ExecuteMsg::Execute {
//         msg: ExecuteSubMsg {
//             contract_addr: None,
//             msg: to_binary(&Empty {}).unwrap(),
//             funds_amount: Uint128::from(100_000000u128),
//         },
//         result_token: "BLUNA".to_string(),
//         wanted_profit: Decimal::from_ratio(10u128, 100u128),
//     };

//     let result = execute(deps.as_mut(), env, whitelist_info, exec_msg).expect_err("expect error");

//     assert_eq!(
//         result,
//         ContractError::from(StdError::NotFound {
//             kind: "profit".to_string()
//         })
//     );
// }

// #[test]
// fn throws_if_not_whitelisted_executor() {
//     let (mut deps, env, _owner_info) = setup();

//     let user_info = mock_info("user001", &[]);
//     let whitelist_info = mock_info("whitelisted_exec", &[]);

//     let execute_msg = ExecuteMsg::Execute {
//         msg: ExecuteSubMsg {
//             contract_addr: None,
//             msg: to_binary(&Empty {}).unwrap(),
//             funds_amount: Uint128::from(100_000000u128),
//         },
//         result_token: "BLUNA".to_string(),
//         wanted_profit: Decimal::from_ratio(1u128, 100u128),
//     };

//     let unbond_msg = ExecuteMsg::UnbondLiquidity {};
//     let withdraw_msg = ExecuteMsg::WithdrawLiquidity {};

//     //
//     // NOT WHITELISTED
//     //
//     let result = execute(deps.as_mut(), env.clone(), user_info.clone(), execute_msg.clone())
//         .expect_err("expect error");
//     assert_eq!(result, ContractError::NotWhitelisted {});

//     let result = execute(deps.as_mut(), env.clone(), user_info.clone(), unbond_msg.clone())
//         .expect_err("expect error");
//     assert_eq!(result, ContractError::NotWhitelisted {});

//     let result = execute(deps.as_mut(), env.clone(), user_info, withdraw_msg.clone())
//         .expect_err("expect error");
//     assert_eq!(result, ContractError::NotWhitelisted {});

//     //
//     // WHITELISTED
//     //
//     let result = execute(deps.as_mut(), env.clone(), whitelist_info.clone(), execute_msg)
//         .expect_err("expect error");

//     assert_eq!(result, ContractError::NotEnoughFundsTakeable {});

//     let result = execute(deps.as_mut(), env.clone(), whitelist_info.clone(), unbond_msg)
//         .expect_err("expect error");
//     assert_eq!(result, ContractError::NothingToUnbond {});

//     let result =
//         execute(deps.as_mut(), env, whitelist_info, withdraw_msg).expect_err("expect error");
//     assert_eq!(result, ContractError::NothingToWithdraw {});
// }

// #[test]
// fn check_unbonding() {
//     let (mut deps, env, _owner_info) = setup();

//     let pool_params = create_default_pool_params();
//     let whitelist_info = mock_info("whitelisted_exec", &[]);

//     let unbond_msg = ExecuteMsg::UnbondLiquidity {};

//     deps.querier.with_token_balances(&[
//         (
//             &String::from("cluna"),
//             &[(&String::from(MOCK_CONTRACT_ADDR), &Uint128::new(100_000000u128))],
//         ),
//         (
//             &String::from("lunax"),
//             &[(&String::from(MOCK_CONTRACT_ADDR), &Uint128::new(200_000000u128))],
//         ),
//         (
//             &String::from("bluna"),
//             &[(&String::from(MOCK_CONTRACT_ADDR), &Uint128::new(300_000000u128))],
//         ),
//         (
//             &String::from("stluna"),
//             &[(&String::from(MOCK_CONTRACT_ADDR), &Uint128::new(400_000000u128))],
//         ),
//         (
//             &String::from("nluna"),
//             &[(&String::from(MOCK_CONTRACT_ADDR), &Uint128::new(500_000000u128))],
//         ),
//         (
//             &String::from("steak_cw"),
//             &[(&String::from(MOCK_CONTRACT_ADDR), &Uint128::new(600_000000u128))],
//         ),
//     ]);

//     let result =
//         execute(deps.as_mut(), env, whitelist_info, unbond_msg).expect("expected response");
//     assert_eq!(
//         result.attributes,
//         vec![
//             attr("action", "unbond"),
//             attr("token", "BLUNA"),
//             attr("amount", "300000000"),
//             attr("token", "STLUNA"),
//             attr("amount", "400000000"),
//             attr("token", "NLUNA"),
//             attr("amount", "500000000"),
//             attr("token", "CLUNA"),
//             attr("amount", "100000000"),
//             attr("token", "LUNAX"),
//             attr("amount", "200000000"),
//             attr("token", "STEAK"),
//             attr("amount", "600000000"),
//         ]
//     );

//     // 5 + nluna->bluna + steak
//     assert_eq!(result.messages.len(), 7);

//     // bluna
//     match result.messages[0].msg.clone() {
//         CosmosMsg::Wasm(WasmMsg::Execute {
//             funds,
//             contract_addr,
//             msg,
//         }) => {
//             assert_eq!(contract_addr, pool_params.bluna_cw20.to_string());
//             assert_eq!(funds.len(), 0);
//             let sub_msg: Cw20ExecuteMsg = from_binary(&msg).unwrap();

//             assert_eq!(
//                 sub_msg,
//                 Cw20ExecuteMsg::Send {
//                     contract: pool_params.bluna_addr.clone(),
//                     amount: Uint128::from(300_000000u128),
//                     msg: to_binary(&basset::hub::Cw20HookMsg::Unbond {}).unwrap()
//                 }
//             );
//         },
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     // stluna -> same as bluna but just sending stluna to bluna hub
//     match result.messages[1].msg.clone() {
//         CosmosMsg::Wasm(WasmMsg::Execute {
//             funds,
//             contract_addr,
//             msg,
//         }) => {
//             assert_eq!(contract_addr, pool_params.stluna_cw20.to_string());
//             assert_eq!(funds.len(), 0);
//             let sub_msg: Cw20ExecuteMsg = from_binary(&msg).unwrap();

//             assert_eq!(
//                 sub_msg,
//                 Cw20ExecuteMsg::Send {
//                     contract: pool_params.bluna_addr.clone(),
//                     amount: Uint128::from(400_000000u128),
//                     msg: to_binary(&basset::hub::Cw20HookMsg::Unbond {}).unwrap()
//                 }
//             );
//         },
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     // nluna (withdraw bluna from nluna hub) -> bluna
//     match result.messages[2].msg.clone() {
//         CosmosMsg::Wasm(WasmMsg::Execute {
//             funds,
//             contract_addr,
//             msg,
//         }) => {
//             assert_eq!(contract_addr, pool_params.nluna_cw20.to_string());
//             assert_eq!(funds.len(), 0);
//             let sub_msg: Cw20ExecuteMsg = from_binary(&msg).unwrap();

//             assert_eq!(
//                 sub_msg,
//                 Cw20ExecuteMsg::Send {
//                     contract: pool_params.nluna_addr.clone(),
//                     amount: Uint128::from(500_000000u128),
//                     msg: to_binary(&basset_vault::basset_vault::Cw20HookMsg::Withdraw {}).unwrap()
//                 }
//             );
//         },
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     match result.messages[3].msg.clone() {
//         CosmosMsg::Wasm(WasmMsg::Execute {
//             funds,
//             contract_addr,
//             msg,
//         }) => {
//             assert_eq!(contract_addr, pool_params.bluna_cw20.to_string());
//             assert_eq!(funds.len(), 0);
//             let sub_msg: Cw20ExecuteMsg = from_binary(&msg).unwrap();

//             assert_eq!(
//                 sub_msg,
//                 Cw20ExecuteMsg::Send {
//                     contract: pool_params.bluna_addr,
//                     amount: Uint128::from(500_000000u128),
//                     msg: to_binary(&basset::hub::Cw20HookMsg::Unbond {}).unwrap()
//                 }
//             );
//         },
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     // cluna
//     match result.messages[4].msg.clone() {
//         CosmosMsg::Wasm(WasmMsg::Execute {
//             funds,
//             contract_addr,
//             msg,
//         }) => {
//             assert_eq!(contract_addr, pool_params.cluna_cw20.to_string());
//             assert_eq!(funds.len(), 0);
//             let sub_msg: Cw20ExecuteMsg = from_binary(&msg).unwrap();

//             assert_eq!(
//                 sub_msg,
//                 Cw20ExecuteMsg::Send {
//                     contract: pool_params.cluna_addr,
//                     amount: Uint128::from(100_000000u128),
//                     msg: to_binary(&prism_protocol::vault::Cw20HookMsg::Unbond {}).unwrap()
//                 }
//             );
//         },
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     // lunax
//     match result.messages[5].msg.clone() {
//         CosmosMsg::Wasm(WasmMsg::Execute {
//             funds,
//             contract_addr,
//             msg,
//         }) => {
//             assert_eq!(contract_addr, pool_params.lunax_cw20.to_string());
//             assert_eq!(funds.len(), 0);
//             let sub_msg: Cw20ExecuteMsg = from_binary(&msg).unwrap();

//             assert_eq!(
//                 sub_msg,
//                 Cw20ExecuteMsg::Send {
//                     contract: pool_params.lunax_addr,
//                     amount: Uint128::from(200_000000u128),
//                     msg: to_binary(&StaderExecute::QueueUndelegate {}).unwrap()
//                 }
//             );
//         },
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     // steak
//     match result.messages[6].msg.clone() {
//         CosmosMsg::Wasm(WasmMsg::Execute {
//             funds,
//             contract_addr,
//             msg,
//         }) => {
//             assert_eq!(contract_addr, pool_params.steak_cw20.to_string());
//             assert_eq!(funds.len(), 0);
//             let sub_msg: Cw20ExecuteMsg = from_binary(&msg).unwrap();

//             assert_eq!(
//                 sub_msg,
//                 Cw20ExecuteMsg::Send {
//                     contract: pool_params.steak_addr,
//                     amount: Uint128::from(600_000000u128),
//                     msg: to_binary(&SteakReceiveMsg::QueueUnbond {
//                         receiver: None
//                     })
//                     .unwrap()
//                 }
//             );
//         },
//         _ => panic!("DO NOT ENTER HERE"),
//     }
// }

// #[test]
// fn check_withdrawing() {
//     let (mut deps, env, _owner_info) = setup();

//     let pool_params = create_default_pool_params();
//     let whitelist_info = mock_info("whitelisted_exec", &[]);

//     let withdraw_msg = ExecuteMsg::WithdrawLiquidity {};

//     deps.querier.with_withdrawable(Uint128::from(10_000000u128));

//     let result =
//         execute(deps.as_mut(), env, whitelist_info, withdraw_msg).expect("expected response");

//     assert_eq!(
//         result.attributes,
//         vec![
//             attr("action", "withdraw"),
//             attr("token", "BLUNA"),
//             attr("amount", "30000000"),
//             attr("token", "CLUNA"),
//             attr("amount", "10000000"),
//             attr("token", "LUNAX"),
//             attr("amount", "10200000"),
//             attr("token", "STEAK"),
//             attr("amount", "10000000"),
//             // STLUNA not withdrawn as it is in BLUNA
//             // NLUNA not withdrawn as it is in BLUNA
//         ]
//     );

//     // cluna + lunax + bluna
//     assert_eq!(result.messages.len(), 4);

//     // bluna
//     match result.messages[0].msg.clone() {
//         CosmosMsg::Wasm(WasmMsg::Execute {
//             funds,
//             contract_addr,
//             msg,
//         }) => {
//             assert_eq!(contract_addr, pool_params.bluna_addr);
//             assert_eq!(funds.len(), 0);
//             let sub_msg: basset::hub::ExecuteMsg = from_binary(&msg).unwrap();

//             assert_eq!(sub_msg, basset::hub::ExecuteMsg::WithdrawUnbonded {});
//         },
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     // cluna
//     match result.messages[1].msg.clone() {
//         CosmosMsg::Wasm(WasmMsg::Execute {
//             funds,
//             contract_addr,
//             msg,
//         }) => {
//             assert_eq!(contract_addr, pool_params.cluna_addr);
//             assert_eq!(funds.len(), 0);
//             let sub_msg: prism_protocol::vault::ExecuteMsg = from_binary(&msg).unwrap();

//             assert_eq!(sub_msg, prism_protocol::vault::ExecuteMsg::WithdrawUnbonded {});
//         },
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     // lunax
//     match result.messages[2].msg.clone() {
//         CosmosMsg::Wasm(WasmMsg::Execute {
//             funds,
//             contract_addr,
//             msg,
//         }) => {
//             assert_eq!(contract_addr, pool_params.lunax_addr);
//             assert_eq!(funds.len(), 0);
//             let sub_msg: StaderExecute = from_binary(&msg).unwrap();

//             assert_eq!(
//                 sub_msg,
//                 StaderExecute::WithdrawFundsToWallet {
//                     batch_id: 0u64
//                 }
//             );
//         },
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     // steak
//     match result.messages[3].msg.clone() {
//         CosmosMsg::Wasm(WasmMsg::Execute {
//             funds,
//             contract_addr,
//             msg,
//         }) => {
//             assert_eq!(contract_addr, pool_params.steak_addr);
//             assert_eq!(funds.len(), 0);
//             let sub_msg: steak::ExecuteMsg = from_binary(&msg).unwrap();

//             assert_eq!(
//                 sub_msg,
//                 steak::ExecuteMsg::WithdrawUnbonded {
//                     receiver: None
//                 }
//             );
//         },
//         _ => panic!("DO NOT ENTER HERE"),
//     }
// }

// fn _unbonding_slow_120(
// ) -> (OwnedDeps<MockStorage, MockApi, WasmMockQuerier>, Env, MessageInfo, Response) {
//     let (mut deps, _env, _owner_info, _res) = _provide_liquidity_again();

//     // arbs executed and created 2 luna
//     deps.querier.with_balance(&[(
//         &String::from(MOCK_CONTRACT_ADDR),
//         &[Coin {
//             denom: "uluna".to_string(),
//             amount: Uint128::new(100_000000u128 + 120_000000u128),
//         }],
//     )]);

//     let lptoken_cw20 = mock_info("lptoken", &[]);

//     let withdraw = ExecuteMsg::Receive(Cw20ReceiveMsg {
//         amount: Uint128::from(120_000000u128),
//         sender: "user001".to_string(),
//         msg: to_binary(&Cw20HookMsg::Unbond {
//             immediate: Some(false),
//         })
//         .unwrap(),
//     });

//     let res =
//         execute(deps.as_mut(), _env.clone(), lptoken_cw20, withdraw).expect("expected a response");

//     deps.querier.with_token_balances(&[(
//         &String::from("lptoken"),
//         &[(&String::from(MOCK_CONTRACT_ADDR), &Uint128::new(100_000000u128))],
//     )]);

//     (deps, _env, _owner_info, res)
// }

// #[test]
// fn withdrawing_liquidity_success() {
//     let (deps, _env, _owner_info, res) = _unbonding_slow_120();

//     assert_eq!(
//         res.attributes,
//         vec![
//             attr("action", "execute_unbond"),
//             attr("from", "user001"),
//             attr("pool_value", "220000000"),
//             attr("withdraw_amount", "120000000"),
//             attr("receive_amount", "119880000"),
//             attr("protocol_fee", "120000"),
//             attr("new_total_supply", "100000000"),
//             attr("unbond_time", "100"),
//             attr("burnt_amount", "120000000")
//         ]
//     );

//     // withdraw + fee
//     assert_eq!(res.messages.len(), 1);

//     match res.messages[0].msg.clone() {
//         CosmosMsg::Wasm(WasmMsg::Execute {
//             funds,
//             contract_addr,
//             msg,
//         }) => {
//             assert_eq!(contract_addr, "lptoken".to_string());
//             assert_eq!(funds.len(), 0);
//             let sub_msg: Cw20ExecuteMsg = from_binary(&msg).unwrap();

//             assert_eq!(
//                 sub_msg,
//                 Cw20ExecuteMsg::Burn {
//                     amount: Uint128::from(120_000000u128)
//                 }
//             );
//         },
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     // check unbonding history correct
//     let unbonding =
//         query_unbond_requests(deps.as_ref(), _env, "user001".to_string()).expect("expects result");

//     assert_eq!(
//         unbonding,
//         UnbondResponse {
//             requests: vec![UnbondItem {
//                 start_time: 1,
//                 release_time: 1 + 100,
//                 amount_asset: Uint128::from(120_000000u128),
//                 id: 1,
//                 protocol_fee: Uint128::from(120000u128),
//                 pool_fee: Uint128::from(2_400000u128)
//             }],
//             withdrawable: Uint128::from(0u128),
//             unbonding: Uint128::from(120000000u128),
//         }
//     );
// }

// fn _unbonding_slow_with_pool_unbonding(
// ) -> (OwnedDeps<MockStorage, MockApi, WasmMockQuerier>, Env, MessageInfo, Response) {
//     let (mut deps, _env, _owner_info, _res) = _provide_liquidity_again();

//     // arbs executed and created 2 luna
//     deps.querier.with_balance(&[(
//         &String::from(MOCK_CONTRACT_ADDR),
//         &[Coin {
//             denom: "uluna".to_string(),
//             amount: Uint128::new(100_000000u128),
//         }],
//     )]);
//     deps.querier.with_unbonding(Uint128::from(24_000000u128));

//     let lptoken_cw20 = mock_info("lptoken", &[]);

//     let withdraw = ExecuteMsg::Receive(Cw20ReceiveMsg {
//         amount: Uint128::from(120_000000u128),
//         sender: "user001".to_string(),
//         msg: to_binary(&Cw20HookMsg::Unbond {
//             immediate: Some(false),
//         })
//         .unwrap(),
//     });

//     let res =
//         execute(deps.as_mut(), _env.clone(), lptoken_cw20, withdraw).expect("expected a response");

//     deps.querier.with_token_balances(&[(
//         &String::from("lptoken"),
//         &[(&String::from(MOCK_CONTRACT_ADDR), &Uint128::new(100_000000u128))],
//     )]);

//     (deps, _env, _owner_info, res)
// }

// fn get_unbonding_value(set: u128) -> Uint128 {
//     let set = Uint128::from(set);
//     let prism_unbonding = set;
//     let stader_unbonding = Decimal::from_ratio(102u128, 100u128) * set;
//     let anchor_unbonding = Decimal::from_ratio(101u128, 100u128) * set * Uint128::from(2u128);
//     let lido_unbonding = Decimal::from_ratio(103u128, 100u128) * set;
//     let steak_unbonding = set;

//     prism_unbonding + stader_unbonding + lido_unbonding + anchor_unbonding + steak_unbonding
// }
// fn get_withdraw_value(set: u128) -> Uint128 {
//     let set = Uint128::from(set);
//     let prism = set;
//     let stader = Decimal::from_ratio(102u128, 100u128) * set;
//     let anchor = set;
//     let lido = set * Uint128::from(2u128);
//     let steak = set;

//     prism + stader + lido + anchor + steak
// }

// #[test]
// fn withdrawing_liquidity_with_unbonding_success() {
//     let (_deps, _env, _owner_info, res) = _unbonding_slow_with_pool_unbonding();

//     let pool_value = Uint128::from(100_000000u128) + get_unbonding_value(24_000000u128);
//     let expected_asset = pool_value * Decimal::from_ratio(120u128, 220u128);

//     let fee = Decimal::from_str("0.001").unwrap() * expected_asset;
//     let receive = expected_asset - fee;

//     assert_eq!(
//         res.attributes,
//         vec![
//             attr("action", "execute_unbond"),
//             attr("from", "user001"),
//             attr("pool_value", pool_value),
//             attr("withdraw_amount", expected_asset),
//             attr("receive_amount", receive),
//             attr("protocol_fee", fee),
//             attr("new_total_supply", "100000000"),
//             attr("unbond_time", "100"),
//             attr("burnt_amount", "120000000")
//         ]
//     );

//     // withdraw + fee
//     assert_eq!(res.messages.len(), 1);

//     match res.messages[0].msg.clone() {
//         CosmosMsg::Wasm(WasmMsg::Execute {
//             funds,
//             contract_addr,
//             msg,
//         }) => {
//             assert_eq!(contract_addr, "lptoken".to_string());
//             assert_eq!(funds.len(), 0);
//             let sub_msg: Cw20ExecuteMsg = from_binary(&msg).unwrap();

//             assert_eq!(
//                 sub_msg,
//                 Cw20ExecuteMsg::Burn {
//                     amount: Uint128::from(120_000000u128)
//                 }
//             );
//         },
//         _ => panic!("DO NOT ENTER HERE"),
//     }
// }

// #[test]
// fn withdraw_liquidity_immediate_with_unbonding_no_liquidity_throws() {
//     let (mut deps, _env, _owner_info, _res) = _unbonding_slow_with_pool_unbonding();

//     let lptoken_cw20 = mock_info("lptoken", &[]);

//     let withdraw = ExecuteMsg::Receive(Cw20ReceiveMsg {
//         amount: Uint128::from(100_000000u128),
//         sender: "user001".to_string(),
//         msg: to_binary(&Cw20HookMsg::Unbond {
//             immediate: Some(true),
//         })
//         .unwrap(),
//     });

//     let result =
//         execute(deps.as_mut(), _env, lptoken_cw20, withdraw).expect_err("expected an error");

//     // withdraw + fee
//     assert_eq!(result, ContractError::NotEnoughAssetsInThePool {});
// }

// #[test]
// fn withdraw_liquidity_immediate_no_liquidity_throws() {
//     let (mut deps, env, _owner_info, _res) = _provide_liquidity_again();

//     // arbs executed and created 2 luna
//     deps.querier.with_balance(&[(
//         &String::from(MOCK_CONTRACT_ADDR),
//         &[Coin {
//             denom: "uluna".to_string(),
//             amount: Uint128::new(100_000000u128),
//         }],
//     )]);

//     // is some factor of 120 LUNA unbonding + some rewards = 5*24+x
//     deps.querier.with_unbonding(Uint128::from(24_000000u128));

//     let lptoken_cw20 = mock_info("lptoken", &[]);

//     let withdraw = ExecuteMsg::Receive(Cw20ReceiveMsg {
//         amount: Uint128::from(120_000000u128),
//         sender: "user001".to_string(),
//         msg: to_binary(&Cw20HookMsg::Unbond {
//             immediate: Some(true),
//         })
//         .unwrap(),
//     });

//     let result =
//         execute(deps.as_mut(), env, lptoken_cw20, withdraw).expect_err("expected an error");

//     // withdraw + fee
//     assert_eq!(result, ContractError::NotEnoughAssetsInThePool {});
// }

// #[test]
// fn withdraw_liquidity_immediate_success() {
//     let (mut deps, _env, _owner_info, _res) = _provide_liquidity_again();

//     // total_asset: 220
//     // pool made 2 through arbs
//     let total_pool = Uint128::new(100_000000u128 + 120_000000u128 + 2_000000u128);

//     // arbs executed and created 2 luna
//     deps.querier.with_balance(&[(
//         &String::from(MOCK_CONTRACT_ADDR),
//         &[Coin {
//             denom: "uluna".to_string(),
//             amount: total_pool,
//         }],
//     )]);

//     let lptoken_cw20 = mock_info("lptoken", &[]);
//     let user = mock_info("user001", &[]);

//     let withdraw = ExecuteMsg::Receive(Cw20ReceiveMsg {
//         amount: Uint128::from(100_000000u128),
//         sender: "user001".to_string(),
//         msg: to_binary(&Cw20HookMsg::Unbond {
//             immediate: Some(true),
//         })
//         .unwrap(),
//     });

//     let result = execute(deps.as_mut(), _env.clone(), user, withdraw.clone())
//         .expect_err("expected an error");
//     assert_eq!(result, ContractError::Unauthorized {});

//     let result = execute(deps.as_mut(), _env, lptoken_cw20, withdraw).expect("expected a result");

//     let withdraw_pool_amount = Decimal::from_ratio(100u128, 220u128) * total_pool;
//     let pool_fee = Decimal::from_str("0.02").unwrap() * withdraw_pool_amount;
//     let protocol_fee = Decimal::from_str("0.001").unwrap() * withdraw_pool_amount;
//     assert_eq!(
//         result.attributes,
//         vec![
//             attr("action", "execute_withdraw"),
//             attr("from", "cosmos2contract"),
//             attr("receiver", "user001"),
//             attr("withdraw_amount", withdraw_pool_amount),
//             attr("receive_amount", withdraw_pool_amount - pool_fee - protocol_fee),
//             attr("protocol_fee", protocol_fee),
//             attr("pool_fee", pool_fee),
//             attr("burnt_amount", "100000000")
//         ]
//     );

//     // withdraw + fee + burn
//     assert_eq!(result.messages.len(), 3);

//     match result.messages[0].msg.clone() {
//         CosmosMsg::Bank(BankMsg::Send {
//             to_address,
//             amount,
//         }) => {
//             assert_eq!(to_address, "user001".to_string());
//             assert_eq!(amount.len(), 1);
//             assert_eq!(
//                 amount[0],
//                 Coin {
//                     denom: "uluna".to_string(),
//                     amount: withdraw_pool_amount - pool_fee - protocol_fee
//                 }
//             );
//         },

//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     match result.messages[1].msg.clone() {
//         CosmosMsg::Bank(BankMsg::Send {
//             to_address,
//             amount,
//         }) => {
//             assert_eq!(to_address, "fee".to_string());
//             assert_eq!(amount.len(), 1);
//             assert_eq!(
//                 amount[0],
//                 Coin {
//                     denom: "uluna".to_string(),
//                     amount: protocol_fee
//                 }
//             );
//         },

//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     match result.messages[2].msg.clone() {
//         CosmosMsg::Wasm(WasmMsg::Execute {
//             contract_addr,
//             funds,
//             msg,
//         }) => {
//             assert_eq!(contract_addr, "lptoken".to_string());
//             assert_eq!(funds.len(), 0);

//             let sub_msg: Cw20ExecuteMsg = from_binary(&msg).unwrap();

//             assert_eq!(
//                 sub_msg,
//                 Cw20ExecuteMsg::Burn {
//                     amount: Uint128::from(100_000000u128)
//                 }
//             );
//         },

//         _ => panic!("DO NOT ENTER HERE"),
//     }
// }

// #[test]
// fn withdraw_liquidity_unbonding_query_requests_success() {
//     let (mut deps, _env, _owner_info, _res) = _unbonding_slow_120();

//     //
//     // UNBONDING AGAIN WITH OTHER TIME
//     //

//     let lptoken_cw20 = mock_info("lptoken", &[]);
//     let user = mock_info("user001", &[]);
//     let mid_time = mock_env_51();
//     let end_time = mock_env_200();

//     let unbonding_again = ExecuteMsg::Receive(Cw20ReceiveMsg {
//         amount: Uint128::from(10_000000u128),
//         sender: "user001".to_string(),
//         msg: to_binary(&Cw20HookMsg::Unbond {
//             immediate: Some(false),
//         })
//         .unwrap(),
//     });

//     let res = execute(deps.as_mut(), mid_time.clone(), lptoken_cw20, unbonding_again)
//         .expect("expected a response");

//     assert_eq!(
//         res.attributes,
//         vec![
//             attr("action", "execute_unbond"),
//             attr("from", "user001"),
//             attr("pool_value", "100000000"),
//             attr("withdraw_amount", "10000000"),
//             attr("receive_amount", "9990000"),
//             attr("protocol_fee", "10000"),
//             attr("new_total_supply", "90000000"),
//             attr("unbond_time", "100"),
//             attr("burnt_amount", "10000000")
//         ]
//     );

//     let unbonding = query_unbond_requests(deps.as_ref(), mid_time.clone(), "user001".to_string())
//         .expect("expects result");

//     assert_eq!(
//         unbonding,
//         UnbondResponse {
//             requests: vec![
//                 UnbondItem {
//                     start_time: 1,
//                     release_time: 1 + 100,
//                     amount_asset: Uint128::from(120_000000u128),
//                     id: 1,
//                     protocol_fee: Uint128::from(120000u128),
//                     pool_fee: Uint128::from(1_200000u128),
//                 },
//                 UnbondItem {
//                     start_time: 51,
//                     release_time: 51 + 100,
//                     amount_asset: Uint128::from(10_000000u128),
//                     id: 2,
//                     protocol_fee: Uint128::from(10000u128),
//                     pool_fee: Uint128::from(200000u128),
//                 }
//             ],
//             withdrawable: Uint128::from(0u128),
//             unbonding: Uint128::from(130000000u128)
//         },
//     );

//     let share = query_share(deps.as_ref(), _env.clone());
//     //
//     // WITHDRAW IMMEDIATE
//     //
//     let withdraw_immediate = ExecuteMsg::WithdrawImmediate {
//         id: 1,
//     };

//     let res = execute(deps.as_mut(), mid_time.clone(), user.clone(), withdraw_immediate)
//         .expect("expected a response");

//     let withdraw_pool_amount = Uint128::new(120_000000u128);
//     let pool_fee = Decimal::from_str("0.02").unwrap()
//         * withdraw_pool_amount
//         * Decimal::from_str("0.5").unwrap();
//     let protocol_fee = Decimal::from_str("0.001").unwrap() * withdraw_pool_amount;
//     let receive_amount = withdraw_pool_amount - pool_fee - protocol_fee;

//     assert_eq!(
//         res.attributes,
//         vec![
//             attr("action", "execute_withdraw"),
//             attr("from", "cosmos2contract"),
//             attr("receiver", "user001"),
//             attr("withdraw_amount", withdraw_pool_amount),
//             attr("receive_amount", receive_amount),
//             attr("protocol_fee", protocol_fee),
//             attr("pool_fee", pool_fee),
//             // no burn, as it already happend during normal withdraw
//             // attr("burnt_amount", "100000000")
//         ]
//     );

//     // withdraw + fee (without burn)
//     assert_eq!(res.messages.len(), 2);

//     match res.messages[0].msg.clone() {
//         CosmosMsg::Bank(BankMsg::Send {
//             to_address,
//             amount,
//         }) => {
//             assert_eq!(to_address, "user001".to_string());
//             assert_eq!(amount.len(), 1);
//             assert_eq!(
//                 amount[0],
//                 Coin {
//                     denom: "uluna".to_string(),
//                     amount: receive_amount
//                 }
//             );
//         },

//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     match res.messages[1].msg.clone() {
//         CosmosMsg::Bank(BankMsg::Send {
//             to_address,
//             amount,
//         }) => {
//             assert_eq!(to_address, "fee".to_string());
//             assert_eq!(amount.len(), 1);
//             assert_eq!(
//                 amount[0],
//                 Coin {
//                     denom: "uluna".to_string(),
//                     amount: protocol_fee
//                 }
//             );
//         },

//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     let unbonding = query_unbond_requests(deps.as_ref(), mid_time, "user001".to_string())
//         .expect("expects result");

//     assert_eq!(
//         unbonding,
//         UnbondResponse {
//             requests: vec![UnbondItem {
//                 start_time: 51,
//                 release_time: 51 + 100,
//                 amount_asset: Uint128::from(10_000000u128),
//                 id: 2,
//                 protocol_fee: Uint128::from(10000u128),
//                 pool_fee: Uint128::from(200000u128)
//             }],
//             withdrawable: Uint128::from(0u128),
//             unbonding: Uint128::from(10_000000u128)
//         }
//     );

//     deps.querier.with_balance(&[(
//         &String::from(MOCK_CONTRACT_ADDR),
//         &[Coin {
//             denom: "uluna".to_string(),
//             amount: Uint128::from(220_000000u128) - receive_amount - protocol_fee,
//         }],
//     )]);

//     // share value is increased by the half protocol fee (share is 50 / 100)
//     let share2 = query_share(deps.as_ref(), _env.clone());
//     assert_eq!(share + pool_fee * Decimal::from_str("0.5").unwrap(), share2);

//     //
//     // WITHDRAW IMMEDIATE AFTER END
//     //
//     let unbonding = query_unbond_requests(deps.as_ref(), end_time.clone(), "user001".to_string())
//         .expect("expects result");

//     assert_eq!(
//         unbonding,
//         UnbondResponse {
//             requests: vec![UnbondItem {
//                 start_time: 51,
//                 release_time: 51 + 100,
//                 amount_asset: Uint128::from(10_000000u128),
//                 id: 2,
//                 protocol_fee: Uint128::from(10000u128),
//                 pool_fee: Uint128::from(0u128)
//             }],
//             withdrawable: Uint128::from(10000000u128),
//             unbonding: Uint128::from(0u128)
//         }
//     );

//     let withdraw_immediate = ExecuteMsg::WithdrawImmediate {
//         id: 2,
//     };

//     let res = execute(deps.as_mut(), end_time.clone(), user, withdraw_immediate)
//         .expect("expected a response");

//     let withdraw_pool_amount = Uint128::new(10_000000u128);
//     let pool_fee2 = Uint128::zero();
//     let protocol_fee2 = Decimal::from_str("0.001").unwrap() * withdraw_pool_amount;
//     let receive_amount2 = withdraw_pool_amount - pool_fee2 - protocol_fee2;

//     assert_eq!(
//         res.attributes,
//         vec![
//             attr("action", "execute_withdraw"),
//             attr("from", "cosmos2contract"),
//             attr("receiver", "user001"),
//             attr("withdraw_amount", withdraw_pool_amount),
//             attr("receive_amount", receive_amount2),
//             attr("protocol_fee", protocol_fee2),
//             attr("pool_fee", pool_fee2),
//             // no burn, as it already happend during normal withdraw
//             // attr("burnt_amount", "100000000")
//         ]
//     );

//     // withdraw + fee (without burn)
//     assert_eq!(res.messages.len(), 2);

//     match res.messages[0].msg.clone() {
//         CosmosMsg::Bank(BankMsg::Send {
//             to_address,
//             amount,
//         }) => {
//             assert_eq!(to_address, "user001".to_string());
//             assert_eq!(amount.len(), 1);
//             assert_eq!(
//                 amount[0],
//                 Coin {
//                     denom: "uluna".to_string(),
//                     amount: receive_amount2
//                 }
//             );
//         },

//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     match res.messages[1].msg.clone() {
//         CosmosMsg::Bank(BankMsg::Send {
//             to_address,
//             amount,
//         }) => {
//             assert_eq!(to_address, "fee".to_string());
//             assert_eq!(amount.len(), 1);
//             assert_eq!(
//                 amount[0],
//                 Coin {
//                     denom: "uluna".to_string(),
//                     amount: protocol_fee2
//                 }
//             );
//         },

//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     let unbonding = query_unbond_requests(deps.as_ref(), end_time, "user001".to_string())
//         .expect("expects result");

//     assert_eq!(
//         unbonding,
//         UnbondResponse {
//             requests: vec![],
//             withdrawable: Uint128::from(0u128),
//             unbonding: Uint128::from(0u128)
//         }
//     );

//     deps.querier.with_balance(&[(
//         &String::from(MOCK_CONTRACT_ADDR),
//         &[Coin {
//             denom: "uluna".to_string(),
//             amount: Uint128::from(220_000000u128)
//                 - receive_amount
//                 - protocol_fee
//                 - receive_amount2
//                 - protocol_fee2,
//         }],
//     )]);

//     let share3 = query_share(deps.as_ref(), _env);
//     // share is not allowed to change by withdrawing after the end time
//     assert_eq!(share2, share3);
// }

// #[test]
// fn withdraw_liquidity_unbonded_all_success() {
//     let (mut deps, _env, _owner_info, _res) = _unbonding_slow_120();

//     //
//     // UNBONDING AGAIN WITH OTHER TIME
//     //

//     let lptoken_cw20 = mock_info("lptoken", &[]);
//     let user = mock_info("user001", &[]);
//     let mid_time = mock_env_51();
//     let end_time = mock_env_200();

//     let unbonding_again = ExecuteMsg::Receive(Cw20ReceiveMsg {
//         amount: Uint128::from(10_000000u128),
//         sender: "user001".to_string(),
//         msg: to_binary(&Cw20HookMsg::Unbond {
//             immediate: Some(false),
//         })
//         .unwrap(),
//     });

//     let _res = execute(deps.as_mut(), mid_time.clone(), lptoken_cw20, unbonding_again)
//         .expect("expected a response");

//     let unbonding = query_unbond_requests(deps.as_ref(), end_time.clone(), "user001".to_string())
//         .expect("expects result");

//     assert_eq!(
//         unbonding,
//         UnbondResponse {
//             requests: vec![
//                 UnbondItem {
//                     start_time: 1,
//                     release_time: 1 + 100,
//                     amount_asset: Uint128::from(120_000000u128),
//                     id: 1,
//                     protocol_fee: Uint128::from(120000u128),
//                     pool_fee: Uint128::from(0_000000u128),
//                 },
//                 UnbondItem {
//                     start_time: 51,
//                     release_time: 51 + 100,
//                     amount_asset: Uint128::from(10_000000u128),
//                     id: 2,
//                     protocol_fee: Uint128::from(10000u128),
//                     pool_fee: Uint128::from(0_000000u128),
//                 }
//             ],
//             withdrawable: Uint128::from(130000000u128),
//             unbonding: Uint128::from(0u128)
//         }
//     );

//     let withdrawable =
//         query_withdrawable_unbonded(deps.as_ref(), mid_time.clone(), "user001".to_string())
//             .unwrap();
//     assert_eq!(
//         withdrawable,
//         WithdrawableResponse {
//             withdrawable: Uint128::zero()
//         }
//     );

//     let withdrawable =
//         query_withdrawable_unbonded(deps.as_ref(), end_time.clone(), "user001".to_string())
//             .unwrap();
//     assert_eq!(
//         withdrawable,
//         WithdrawableResponse {
//             withdrawable: Uint128::from(120_000000u128) + Uint128::from(10_000000u128)
//         }
//     );

//     let share = query_share(deps.as_ref(), _env.clone());

//     //
//     // WITHDRAW UNBONDED FAILED
//     //
//     let withdraw_unbonded = ExecuteMsg::WithdrawUnbonded {};

//     let res = execute(deps.as_mut(), mid_time, user.clone(), withdraw_unbonded.clone())
//         .expect_err("expect error");

//     assert_eq!(res, ContractError::NoWithdrawableAsset {});

//     //
//     // WITHDRAW UNBONDED
//     //
//     let res = execute(deps.as_mut(), end_time.clone(), user.clone(), withdraw_unbonded)
//         .expect("expect response");

//     let withdraw_pool_amount = withdrawable.withdrawable;
//     let pool_fee = Uint128::zero();
//     let protocol_fee = Decimal::from_str("0.001").unwrap() * withdraw_pool_amount;
//     let receive_amount = withdraw_pool_amount - pool_fee - protocol_fee;

//     assert_eq!(
//         res.attributes,
//         vec![
//             attr("action", "execute_withdraw"),
//             attr("from", "cosmos2contract"),
//             attr("receiver", "user001"),
//             attr("withdraw_amount", withdraw_pool_amount),
//             attr("receive_amount", receive_amount),
//             attr("protocol_fee", protocol_fee),
//             attr("pool_fee", pool_fee),
//             // no burn, as it already happend during normal withdraw
//             // attr("burnt_amount", "100000000")
//         ]
//     );

//     // withdraw + fee (without burn)
//     assert_eq!(res.messages.len(), 2);

//     match res.messages[0].msg.clone() {
//         CosmosMsg::Bank(BankMsg::Send {
//             to_address,
//             amount,
//         }) => {
//             assert_eq!(to_address, "user001".to_string());
//             assert_eq!(amount.len(), 1);
//             assert_eq!(
//                 amount[0],
//                 Coin {
//                     denom: "uluna".to_string(),
//                     amount: receive_amount
//                 }
//             );
//         },

//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     match res.messages[1].msg.clone() {
//         CosmosMsg::Bank(BankMsg::Send {
//             to_address,
//             amount,
//         }) => {
//             assert_eq!(to_address, "fee".to_string());
//             assert_eq!(amount.len(), 1);
//             assert_eq!(
//                 amount[0],
//                 Coin {
//                     denom: "uluna".to_string(),
//                     amount: protocol_fee
//                 }
//             );
//         },

//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     deps.querier.with_balance(&[(
//         &String::from(MOCK_CONTRACT_ADDR),
//         &[Coin {
//             denom: "uluna".to_string(),
//             amount: Uint128::from(220_000000u128) - receive_amount - protocol_fee,
//         }],
//     )]);

//     // share value is not changed, as there is no pool fee
//     let share2 = query_share(deps.as_ref(), _env);
//     assert_eq!(share, share2);

//     // nothing to withdraw afterwards
//     let withdrawable =
//         query_withdrawable_unbonded(deps.as_ref(), end_time.clone(), "user001".to_string())
//             .unwrap();
//     assert_eq!(
//         withdrawable,
//         WithdrawableResponse {
//             withdrawable: Uint128::zero(),
//         }
//     );

//     let unbonding = query_unbond_requests(deps.as_ref(), end_time.clone(), "user001".to_string())
//         .expect("expects result");

//     // no items
//     assert_eq!(
//         unbonding,
//         UnbondResponse {
//             requests: vec![],
//             withdrawable: Uint128::from(0u128),
//             unbonding: Uint128::from(0u128)
//         }
//     );

//     //
//     // WITHDRAW UNBONDED FAILED
//     //
//     let withdraw_unbonded = ExecuteMsg::WithdrawUnbonded {};

//     let res = execute(deps.as_mut(), end_time, user, withdraw_unbonded).expect_err("expect error");

//     assert_eq!(res, ContractError::NoWithdrawableAsset {});
// }

// #[test]
// fn withdraw_liquidity_unbonded_half_success() {
//     let (mut deps, _env, _owner_info, _res) = _unbonding_slow_120();

//     // difference is that we only unbond part of the history instead of everything
//     //
//     // UNBONDING AGAIN WITH OTHER TIME
//     //

//     let lptoken_cw20 = mock_info("lptoken", &[]);
//     let user = mock_info("user001", &[]);
//     let mid_time = mock_env_51();
//     let before_end_time = mock_env_130();
//     let end_time = mock_env_200();

//     let unbonding_again = ExecuteMsg::Receive(Cw20ReceiveMsg {
//         amount: Uint128::from(10_000000u128),
//         sender: "user001".to_string(),
//         msg: to_binary(&Cw20HookMsg::Unbond {
//             immediate: Some(false),
//         })
//         .unwrap(),
//     });

//     let _res = execute(deps.as_mut(), mid_time, lptoken_cw20, unbonding_again)
//         .expect("expected a response");

//     let unbonding = query_unbond_requests(deps.as_ref(), end_time.clone(), "user001".to_string())
//         .expect("expects result");

//     assert_eq!(
//         unbonding,
//         UnbondResponse {
//             requests: vec![
//                 UnbondItem {
//                     start_time: 1,
//                     release_time: 1 + 100,
//                     amount_asset: Uint128::from(120_000000u128),
//                     id: 1,
//                     protocol_fee: Uint128::from(120000u128),
//                     pool_fee: Uint128::from(0u128),
//                 },
//                 UnbondItem {
//                     start_time: 51,
//                     release_time: 51 + 100,
//                     amount_asset: Uint128::from(10_000000u128),
//                     id: 2,
//                     protocol_fee: Uint128::from(10000u128),
//                     pool_fee: Uint128::from(0u128),
//                 }
//             ],
//             withdrawable: Uint128::from(130000000u128),
//             unbonding: Uint128::from(0u128)
//         }
//     );

//     let withdrawable =
//         query_withdrawable_unbonded(deps.as_ref(), before_end_time.clone(), "user001".to_string())
//             .unwrap();
//     assert_eq!(
//         withdrawable,
//         WithdrawableResponse {
//             withdrawable: Uint128::from(120_000000u128),
//         }
//     );

//     let share = query_share(deps.as_ref(), _env.clone());

//     //
//     // WITHDRAW UNBONDED
//     //
//     let withdraw_unbonded = ExecuteMsg::WithdrawUnbonded {};
//     let res = execute(deps.as_mut(), before_end_time.clone(), user.clone(), withdraw_unbonded)
//         .expect("expect response");

//     let withdraw_pool_amount = withdrawable.withdrawable;
//     let pool_fee = Uint128::zero();
//     let protocol_fee = Decimal::from_str("0.001").unwrap() * withdraw_pool_amount;
//     let receive_amount = withdraw_pool_amount - pool_fee - protocol_fee;

//     assert_eq!(
//         res.attributes,
//         vec![
//             attr("action", "execute_withdraw"),
//             attr("from", "cosmos2contract"),
//             attr("receiver", "user001"),
//             attr("withdraw_amount", withdraw_pool_amount),
//             attr("receive_amount", receive_amount),
//             attr("protocol_fee", protocol_fee),
//             attr("pool_fee", pool_fee),
//             // no burn, as it already happend during normal withdraw
//             // attr("burnt_amount", "100000000")
//         ]
//     );

//     // withdraw + fee (without burn)
//     assert_eq!(res.messages.len(), 2);

//     match res.messages[0].msg.clone() {
//         CosmosMsg::Bank(BankMsg::Send {
//             to_address,
//             amount,
//         }) => {
//             assert_eq!(to_address, "user001".to_string());
//             assert_eq!(amount.len(), 1);
//             assert_eq!(
//                 amount[0],
//                 Coin {
//                     denom: "uluna".to_string(),
//                     amount: receive_amount
//                 }
//             );
//         },

//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     match res.messages[1].msg.clone() {
//         CosmosMsg::Bank(BankMsg::Send {
//             to_address,
//             amount,
//         }) => {
//             assert_eq!(to_address, "fee".to_string());
//             assert_eq!(amount.len(), 1);
//             assert_eq!(
//                 amount[0],
//                 Coin {
//                     denom: "uluna".to_string(),
//                     amount: protocol_fee
//                 }
//             );
//         },

//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     deps.querier.with_balance(&[(
//         &String::from(MOCK_CONTRACT_ADDR),
//         &[Coin {
//             denom: "uluna".to_string(),
//             amount: Uint128::from(220_000000u128) - receive_amount - protocol_fee,
//         }],
//     )]);

//     // share value is not changed, as there is no pool fee
//     let share2 = query_share(deps.as_ref(), _env);
//     assert_eq!(share, share2);

//     // one remaining withdraw
//     let withdrawable =
//         query_withdrawable_unbonded(deps.as_ref(), end_time.clone(), "user001".to_string())
//             .unwrap();
//     assert_eq!(
//         withdrawable,
//         WithdrawableResponse {
//             withdrawable: Uint128::from(10_000000u128)
//         }
//     );

//     let unbonding = query_unbond_requests(deps.as_ref(), end_time, "user001".to_string())
//         .expect("expects result");

//     // 1 item
//     assert_eq!(
//         unbonding,
//         UnbondResponse {
//             requests: vec![UnbondItem {
//                 start_time: 51,
//                 release_time: 51 + 100,
//                 amount_asset: Uint128::from(10_000000u128),
//                 id: 2,
//                 protocol_fee: Uint128::from(10000u128),
//                 pool_fee: Uint128::from(0u128),
//             }],
//             withdrawable: Uint128::from(10000000u128),
//             unbonding: Uint128::from(0u128)
//         }
//     );

//     //
//     // WITHDRAW UNBONDED FAILED
//     //
//     let withdraw_unbonded = ExecuteMsg::WithdrawUnbonded {};

//     let res =
//         execute(deps.as_mut(), before_end_time, user, withdraw_unbonded).expect_err("expect error");

//     assert_eq!(res, ContractError::NoWithdrawableAsset {});
// }

// #[test]
// fn query_check_balances() {
//     let (mut deps, env, _owner_info, _res) = _unbonding_slow_120();

//     deps.querier.with_unbonding(Uint128::from(24_000000u128));
//     deps.querier.with_withdrawable(Uint128::from(10_000000u128));

//     let pool_available = Uint128::from(220_000000u128);
//     let locked = Uint128::from(120_000000u128);
//     let pool_takeable = pool_available - locked;
//     let unbonding = get_unbonding_value(24_000000u128);
//     let withdrawable = get_withdraw_value(10_000000u128);

//     let total_value = pool_available + unbonding + withdrawable - locked;

//     let balance = query_balances(deps.as_ref(), env.clone(), None).unwrap();
//     assert_eq!(
//         balance,
//         BalancesResponse {
//             locked,
//             total_value,
//             pool_available,
//             pool_takeable,
//             unbonding,
//             withdrawable,
//             claims: None,
//             takeable_steps: None,
//             total_lp_supply: None
//         }
//     );

//     let balance_detail = query_balance_details(deps.as_ref(), env, None).unwrap();
//     assert_eq!(
//         balance_detail,
//         BalancesResponse {
//             locked,
//             total_value,
//             pool_available,
//             pool_takeable,
//             unbonding,
//             withdrawable,
//             claims: Some(vec![
//                 ClaimBalance {
//                     token: "BLUNA".to_string(),
//                     withdrawable: Uint128::from(30_000000u128),
//                     // 1.01
//                     unbonding: Uint128::from(48_480000u128)
//                 },
//                 ClaimBalance {
//                     token: "STLUNA".to_string(),
//                     // through bluna
//                     withdrawable: Uint128::from(0u128),
//                     // 1.02
//                     unbonding: Uint128::from(24_720000u128)
//                 },
//                 ClaimBalance {
//                     token: "NLUNA".to_string(),
//                     // through bluna
//                     withdrawable: Uint128::from(0u128),
//                     // through bluna
//                     unbonding: Uint128::from(0u128)
//                 },
//                 ClaimBalance {
//                     token: "CLUNA".to_string(),
//                     withdrawable: Uint128::from(10_000000u128),
//                     unbonding: Uint128::from(24_000000u128),
//                 },
//                 ClaimBalance {
//                     token: "LUNAX".to_string(),
//                     withdrawable: Uint128::from(10_200000u128),
//                     unbonding: Uint128::from(24_480000u128)
//                 },
//                 ClaimBalance {
//                     token: "STEAK".to_string(),
//                     withdrawable: Uint128::from(10_000000u128),
//                     unbonding: Uint128::from(24_000000u128)
//                 },
//             ]),

//             takeable_steps: Some(vec![
//                 (
//                     // 1% = 50% of pool
//                     Decimal::from_ratio(10u128, 1000u128),
//                     Uint128::from(0u128),
//                 ),
//                 (
//                     // 1% = 50% of pool
//                     Decimal::from_ratio(15u128, 1000u128),
//                     Uint128::from(8236000u128),
//                 ),
//                 (
//                     // 1% = 50% of pool
//                     Decimal::from_ratio(20u128, 1000u128),
//                     Uint128::from(69412000u128),
//                 ),
//                 (
//                     // 1% = 50% of pool
//                     Decimal::from_ratio(25u128, 1000u128),
//                     Uint128::from(100000000u128),
//                 ),
//             ]),
//             total_lp_supply: Some(Uint128::from(100000000u128))
//         }
//     );
// }

// #[test]
// fn query_check_available() {
//     let (mut deps, env, _owner_info, _res) = _unbonding_slow_120();
//     deps.querier.with_unbonding(Uint128::from(24_000000u128));
//     deps.querier.with_withdrawable(Uint128::from(10_000000u128));

//     let pool_available = Uint128::from(220_000000u128);
//     let locked = Uint128::from(120_000000u128);
//     let pool_takeable = pool_available - locked;
//     let unbonding = get_unbonding_value(24_000000u128);
//     let withdrawable = get_withdraw_value(10_000000u128);

//     let total_value = pool_available + unbonding + withdrawable - locked;

//     let available = query_takeable(deps.as_ref(), env.clone(), None).expect("expects result");

//     // println!(
//     //     "takeable 0.5: {}",
//     //     calc_takeable(total_value.clone(), pool_takeable.clone(), "0.5")
//     // );
//     // println!(
//     //     "takeable 0.7: {}",
//     //     calc_takeable(total_value.clone(), pool_takeable.clone(), "0.7")
//     // );

//     assert_eq!(
//         available,
//         TakeableResponse {
//             takeable: None,
//             steps: vec![
//                 // 50%
//                 (
//                     Decimal::from_ratio(10u128, 1000u128),
//                     calc_takeable(total_value, pool_takeable, "0.5")
//                 ),
//                 // 70%
//                 (
//                     Decimal::from_ratio(15u128, 1000u128),
//                     calc_takeable(total_value, pool_takeable, "0.7")
//                 ),
//                 // 90%
//                 (
//                     Decimal::from_ratio(20u128, 1000u128),
//                     calc_takeable(total_value, pool_takeable, "0.9")
//                 ),
//                 (
//                     Decimal::from_ratio(25u128, 1000u128),
//                     calc_takeable(total_value, pool_takeable, "1.0")
//                 ),
//             ],
//         },
//     );

//     let available =
//         query_takeable(deps.as_ref(), env.clone(), Some(Decimal::from_str("0.01").unwrap()))
//             .expect("expects result");

//     assert_eq!(
//         available,
//         TakeableResponse {
//             takeable: Some(calc_takeable(total_value, pool_takeable, "0.5")),
//             steps: vec![
//                 // 50%
//                 (
//                     Decimal::from_ratio(10u128, 1000u128),
//                     calc_takeable(total_value, pool_takeable, "0.5")
//                 ),
//                 // 70%
//                 (
//                     Decimal::from_ratio(15u128, 1000u128),
//                     calc_takeable(total_value, pool_takeable, "0.7")
//                 ),
//                 // 90%
//                 (
//                     Decimal::from_ratio(20u128, 1000u128),
//                     calc_takeable(total_value, pool_takeable, "0.9")
//                 ),
//                 (
//                     Decimal::from_ratio(25u128, 1000u128),
//                     calc_takeable(total_value, pool_takeable, "1.0")
//                 ),
//             ],
//         },
//     );

//     let available = query_takeable(deps.as_ref(), env, Some(Decimal::from_str("0.6").unwrap()))
//         .expect_err("expects error");

//     // currently no interpolation possible
//     assert_eq!(
//         available,
//         StdError::NotFound {
//             kind: "profit".to_string()
//         }
//     );
// }

// #[test]
// fn execute_arb() {
//     let (mut deps, env, _owner_info, _res) = _unbonding_slow_120();
//     deps.querier.with_unbonding(Uint128::from(24_000000u128));
//     deps.querier.with_withdrawable(Uint128::from(10_000000u128));

//     let pool_available = Uint128::from(220_000000u128);
//     let locked = Uint128::from(120_000000u128);
//     let _pool_takeable = pool_available - locked;
//     let unbonding = get_unbonding_value(24_000000u128);
//     let withdrawable = get_withdraw_value(10_000000u128);

//     let total_value = pool_available + unbonding + withdrawable - locked;

//     let start_share = query_share(deps.as_ref(), env.clone());
//     assert_eq!(start_share, Uint128::from(152_940000u128));

//     let whitelist_info = mock_info("whitelisted_exec", &[]);
//     let user_info = mock_info("user", &[]);
//     let contract_info = mock_info(MOCK_CONTRACT_ADDR, &[]);

//     let exec_msg = ExecuteMsg::Execute {
//         msg: ExecuteSubMsg {
//             contract_addr: None,
//             funds_amount: Uint128::from(1000_000000u128),
//             msg: to_binary("exec_any_swap").unwrap(),
//         },
//         result_token: "BLUNA".to_string(),
//         wanted_profit: Decimal::from_str("0.025").unwrap(),
//     };
//     let res = execute(deps.as_mut(), env.clone(), whitelist_info.clone(), exec_msg)
//         .expect_err("expects error");
//     assert_eq!(res, ContractError::NotEnoughFundsTakeable {});

//     let exec_msg = ExecuteMsg::Execute {
//         msg: ExecuteSubMsg {
//             contract_addr: None,
//             funds_amount: Uint128::from(10_000000u128),
//             msg: to_binary("exec_any_swap").unwrap(),
//         },
//         result_token: "XXX".to_string(),
//         wanted_profit: Decimal::from_str("0.025").unwrap(),
//     };
//     let res = execute(deps.as_mut(), env.clone(), whitelist_info.clone(), exec_msg)
//         .expect_err("expects error");
//     assert_eq!(res, ContractError::AssetUnknown {});

//     let exec_msg = ExecuteMsg::Execute {
//         msg: ExecuteSubMsg {
//             contract_addr: None,
//             funds_amount: Uint128::zero(),
//             msg: to_binary("exec_any_swap").unwrap(),
//         },
//         result_token: "BLUNA".to_string(),
//         wanted_profit: Decimal::from_str("0.025").unwrap(),
//     };
//     let res = execute(deps.as_mut(), env.clone(), whitelist_info.clone(), exec_msg)
//         .expect_err("expects error");
//     assert_eq!(res, ContractError::InvalidZeroAmount {});

//     let res = execute(
//         deps.as_mut(),
//         env.clone(),
//         contract_info.clone(),
//         ExecuteMsg::AssertResult {
//             result_token: "BLUNA".to_string(),
//             wanted_profit: Decimal::from_str("0.01").unwrap(),
//         },
//     )
//     .expect_err("expect error");

//     assert_eq!(res, ContractError::NotExecuting {});

//     let wanted_profit = Decimal::from_str("0.015").unwrap();
//     let takeable = query_takeable(deps.as_ref(), env.clone(), Some(wanted_profit))
//         .expect("expects result")
//         .takeable
//         .expect("expects takeable");

//     println!("Taking: {:?}", takeable);

//     let exec_msg = ExecuteMsg::Execute {
//         msg: ExecuteSubMsg {
//             contract_addr: None,
//             funds_amount: takeable,
//             msg: to_binary("exec_any_swap").unwrap(),
//         },
//         result_token: "BLUNA".to_string(),
//         wanted_profit,
//     };
//     let res = execute(deps.as_mut(), env.clone(), whitelist_info.clone(), exec_msg)
//         .expect("expects response");

//     assert_eq!(res.attributes, vec![attr("action", "execute")]);
//     assert_eq!(res.messages.len(), 2);
//     match res.messages[0].msg.clone() {
//         CosmosMsg::Wasm(WasmMsg::Execute {
//             funds,
//             contract_addr,
//             msg,
//         }) => {
//             assert_eq!(contract_addr, whitelist_info.sender.to_string());
//             assert_eq!(
//                 funds,
//                 vec![Coin {
//                     denom: "uluna".to_string(),
//                     amount: takeable
//                 }]
//             );

//             let sub_msg: String = from_binary(&msg).unwrap();
//             assert_eq!(sub_msg, "exec_any_swap");
//         },
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     let sub_msg: ExecuteMsg;
//     match res.messages[1].msg.clone() {
//         CosmosMsg::Wasm(WasmMsg::Execute {
//             funds,
//             contract_addr,
//             msg,
//         }) => {
//             assert_eq!(contract_addr, MOCK_CONTRACT_ADDR.to_string());
//             assert_eq!(funds.len(), 0);
//             sub_msg = from_binary(&msg).unwrap();

//             assert_eq!(
//                 sub_msg,
//                 ExecuteMsg::AssertResult {
//                     result_token: "BLUNA".to_string(),
//                     wanted_profit
//                 }
//             );
//         },
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     //
//     // EXPECT PROVIDING LIQUIDITY WHILE EXECUTION TO THROW
//     //

//     let res = execute(
//         deps.as_mut(),
//         env.clone(),
//         user_info,
//         ExecuteMsg::ProvideLiquidity {
//             asset: Asset {
//                 amount: Uint128::from(100u128),
//                 info: AssetInfo::NativeToken {
//                     denom: "uluna".to_string(),
//                 },
//             },
//             receiver: None,
//         },
//     )
//     .expect_err("expect error");

//     assert_eq!(res, ContractError::AlreadyExecuting {});

//     let res = execute(
//         deps.as_mut(),
//         env.clone(),
//         whitelist_info,
//         ExecuteMsg::Execute {
//             msg: ExecuteSubMsg {
//                 contract_addr: None,
//                 msg: to_binary(&Empty {}).unwrap(),
//                 funds_amount: Uint128::from(100u128),
//             },
//             result_token: "LUNAX".to_string(),
//             wanted_profit,
//         },
//     )
//     .expect_err("expect error");

//     assert_eq!(res, ContractError::AlreadyExecuting {});

//     //
//     // APPLYING SUB MSG TO NEW BALANCE
//     //
//     let profit_factor = Uint128::from(1000u128) * (Decimal::one() + wanted_profit);
//     // 100 bluna -> 101 luna
//     let bluna = takeable.multiply_ratio(profit_factor, Uint128::from(1010u128));
//     let unbonding = takeable.multiply_ratio(profit_factor, Uint128::from(1010u128));
//     // let xvalue = unbonding * wanted_profit *
//     let profit = takeable * wanted_profit;

//     let fee_value_expected = profit * Decimal::from_str("0.1").unwrap();

//     // we have taken the takeable amount from the balance
//     deps.querier.with_balance(&[(
//         &String::from(MOCK_CONTRACT_ADDR),
//         &[Coin {
//             denom: "uluna".to_string(),
//             amount: Uint128::new(100_000000u128 + 120_000000u128) - takeable,
//         }],
//     )]);

//     // and received the result in bluna
//     deps.querier.with_token_balances(&[
//         (
//             &String::from("lptoken"),
//             &[(&String::from(MOCK_CONTRACT_ADDR), &Uint128::new(100_000000u128))],
//         ),
//         (&String::from("bluna"), &[(&String::from(MOCK_CONTRACT_ADDR), &bluna)]),
//     ]);
//     //
//     // END APPLYING SUB MSG TO NEW BALANCE
//     //

//     let res =
//         execute(deps.as_mut(), env.clone(), contract_info, sub_msg).expect("expects response");

//     assert_eq!(
//         res.attributes,
//         vec![
//             attr("action", "assert_result"),
//             attr("old_value", total_value.to_string()),
//             attr("new_value", "306003539"),
//             attr("used_balance", takeable.to_string()),
//             attr("xbalance", bluna.to_string()),
//             attr("xfactor", "1.01"),
//             attr("xvalue", Decimal::from_str("1.01").unwrap() * bluna),
//             attr("profit", "123539"),
//             attr("fee_amount", "12353"),
//             attr("fee_minted_lp", "4037"),
//             attr("unbond_token", "BLUNA"),
//             attr("unbond_amount", bluna)
//         ]
//     );

//     //
//     // APPLYING SUB MSG TO NEW BALANCE
//     //
//     deps.querier.with_token_balances(&[(
//         &String::from("lptoken"),
//         &[(&String::from(MOCK_CONTRACT_ADDR), &Uint128::new(100_000000u128 + 10161u128))],
//     )]);
//     // xasset moved to unbonding
//     deps.querier.with_unbonding_bluna(unbonding);
//     //
//     // END APPLYING SUB MSG TO NEW BALANCE
//     //

//     assert_eq!(res.messages.len(), 2);

//     match res.messages[0].msg.clone() {
//         CosmosMsg::Wasm(WasmMsg::Execute {
//             funds,
//             contract_addr,
//             msg,
//         }) => {
//             assert_eq!(contract_addr, "lptoken".to_string());
//             assert_eq!(funds.len(), 0);
//             let sub_msg: Cw20ExecuteMsg = from_binary(&msg).unwrap();

//             if let Cw20ExecuteMsg::Mint {
//                 recipient,
//                 amount,
//             } = sub_msg
//             {
//                 assert_eq!(recipient, "fee");
//                 let fee_value = query_user_info(deps.as_ref(), env.clone(), amount).unwrap();

//                 // check that the result is very close
//                 assert_delta!(fee_value.received_asset, fee_value_expected, Uint128::from(10u128));
//             } else {
//                 panic!("DO NOT ENTER HERE");
//             }
//         },
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     //
//     // EXPECT NEW SHARE TO BE BIGGER
//     //
//     let new_share = query_share(deps.as_ref(), env.clone());

//     assert!(new_share.gt(&start_share), "new share must be bigger than start");
//     assert_eq!(new_share, Uint128::from(152_986224u128));

//     // expect takeable to be 0 afterwards
//     let takeable = query_takeable(deps.as_ref(), env, Some(wanted_profit))
//         .expect("expects result")
//         .takeable
//         .expect("expects takeable");

//     assert_eq!(takeable, Uint128::zero());
// }

// fn calc_takeable(total_value: Uint128, pool_takeable: Uint128, share: &str) -> Uint128 {
//     // total value * share = total pool that can be used for that share
//     // + takeable - total value

//     // Example:
//     // share = 0.7
//     // total_value: 1000
//     // total_value_for_profit 700
//     // pool_takeable: 400
//     // pool_takeable_for_profit -> 100 (total_for_profit+pool_takeable-total)
//     (total_value * Decimal::from_str(share).expect("expect value"))
//         .checked_add(pool_takeable)
//         .unwrap_or(Uint128::zero())
//         .checked_sub(total_value)
//         .unwrap_or(Uint128::zero())
// }

// fn query_share(deps: Deps, env: Env) -> Uint128 {
//     let response = query_user_info(deps, env, Uint128::from(50_000000u128)).unwrap();
//     response.received_asset
// }

// #[test]
// fn test_decimal() {
//     let result = Decimal::from_str("2.0")
//         .unwrap()
//         .checked_mul_dec(Decimal::from_str("2.5").unwrap())
//         .expect("expect result");
//     assert_eq!(result, Decimal::from_str("5.0").unwrap());

//     let result = Decimal::from_str("0.01")
//         .unwrap()
//         .checked_mul_dec(Decimal::from_str("0.02").unwrap())
//         .expect("expect result");
//     assert_eq!(result, Decimal::from_str("0.0002").unwrap());
// }
