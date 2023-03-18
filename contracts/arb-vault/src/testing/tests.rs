use std::str::FromStr;

use crate::{
    contract::execute,
    error::ContractError,
    query::{query_state, query_takeable, query_unbond_requests},
    testing::helpers::{_mock_env_at_timestamp, create_default_lsd_configs, mock_env, setup_test},
};

use crate::query::{query_config, query_user_info};

use astroport::asset::{native_asset, token_asset_info};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::testing::{mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    attr, coin, from_binary, to_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, Deps, OwnedDeps,
    Response, Uint128, WasmMsg,
};
use eris::arb_vault::{
    Balances, ClaimBalance, Config, ConfigResponse, Cw20HookMsg, ExecuteMsg, ExecuteSubMsg,
    FeeConfig, StateDetails, StateResponse, TakeableResponse, UnbondItem, UnbondRequestsResponse,
    UserInfoResponse, UtilizationMethod,
};

use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use itertools::Itertools;

use super::custom_querier::CustomQuerier;

#[cw_serde]
struct Empty {}

#[test]
fn proper_initialization() {
    let deps = setup_test();

    let config: ConfigResponse = query_config(deps.as_ref()).unwrap();

    assert_eq!(
        config,
        ConfigResponse {
            config: Config {
                utoken: "utoken".into(),
                utilization_method: eris::arb_vault::UtilizationMethod::Steps(vec![
                    (Decimal::from_ratio(10u128, 1000u128), Decimal::from_ratio(50u128, 100u128),),
                    (Decimal::from_ratio(15u128, 1000u128), Decimal::from_ratio(70u128, 100u128),),
                    (Decimal::from_ratio(20u128, 1000u128), Decimal::from_ratio(90u128, 100u128),),
                    (Decimal::from_ratio(25u128, 1000u128), Decimal::from_ratio(100u128, 100u128),),
                ]),
                unbond_time_s: 100,
                lp_addr: Addr::unchecked("lptoken"),
                lsds: create_default_lsd_configs()
                    .into_iter()
                    .map(|a| a.validate(deps.as_ref().api).unwrap())
                    .collect_vec()
            },
            fee_config: eris::arb_vault::FeeConfig {
                protocol_fee_contract: Addr::unchecked("fee"),
                protocol_performance_fee: Decimal::from_str("0.01").unwrap(),
                protocol_withdraw_fee: Decimal::from_str("0.02").unwrap(),
                immediate_withdraw_fee: Decimal::from_str("0.05").unwrap(),
            },
            owner: Addr::unchecked("owner"),
        }
    );
}

#[test]
fn update_config() {
    let mut deps = setup_test();

    let upd_msg = ExecuteMsg::UpdateConfig {
        utilization_method: None,
        unbond_time_s: Some(10u64),
        lsds: None,
        fee_config: None,
        set_whitelist: None,
        remove_whitelist: None,
    };

    let res =
        execute(deps.as_mut(), mock_env(), mock_info("user", &[]), upd_msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    let _res = execute(deps.as_mut(), mock_env(), mock_info("owner", &[]), upd_msg).unwrap();

    let config = query_config(deps.as_ref()).unwrap();

    assert_eq!(
        config,
        ConfigResponse {
            config: Config {
                utoken: "utoken".into(),
                utilization_method: UtilizationMethod::Steps(vec![
                    (Decimal::from_ratio(10u128, 1000u128), Decimal::from_ratio(50u128, 100u128),),
                    (Decimal::from_ratio(15u128, 1000u128), Decimal::from_ratio(70u128, 100u128),),
                    (Decimal::from_ratio(20u128, 1000u128), Decimal::from_ratio(90u128, 100u128),),
                    (Decimal::from_ratio(25u128, 1000u128), Decimal::from_ratio(100u128, 100u128),),
                ]),
                unbond_time_s: 10,
                lp_addr: Addr::unchecked("lptoken"),
                lsds: create_default_lsd_configs()
                    .into_iter()
                    .map(|a| a.validate(deps.as_ref().api).unwrap())
                    .collect_vec()
            },
            fee_config: FeeConfig {
                protocol_fee_contract: Addr::unchecked("fee"),
                protocol_performance_fee: Decimal::from_str("0.01").unwrap(),
                protocol_withdraw_fee: Decimal::from_str("0.02").unwrap(),
                immediate_withdraw_fee: Decimal::from_str("0.05").unwrap(),
            },
            owner: Addr::unchecked("owner"),
        }
    );

    let upd_msg = ExecuteMsg::UpdateConfig {
        utilization_method: Some(UtilizationMethod::Steps(vec![])),
        unbond_time_s: None,
        lsds: None,
        fee_config: None,
        remove_whitelist: None,
        set_whitelist: None,
    };

    let _res = execute(deps.as_mut(), mock_env(), mock_info("owner", &[]), upd_msg).unwrap();

    let config = query_config(deps.as_ref()).unwrap();

    assert_eq!(
        config,
        ConfigResponse {
            config: Config {
                utoken: "utoken".into(),
                utilization_method: UtilizationMethod::Steps(vec![]),
                unbond_time_s: 10,
                lp_addr: Addr::unchecked("lptoken"),
                lsds: create_default_lsd_configs()
                    .into_iter()
                    .map(|a| a.validate(deps.as_ref().api).unwrap())
                    .collect_vec()
            },
            fee_config: FeeConfig {
                protocol_fee_contract: Addr::unchecked("fee"),
                protocol_performance_fee: Decimal::from_str("0.01").unwrap(),
                protocol_withdraw_fee: Decimal::from_str("0.02").unwrap(),
                immediate_withdraw_fee: Decimal::from_str("0.05").unwrap(),
            },
            owner: Addr::unchecked("owner"),
        }
    );
}

#[test]
fn provide_liquidity_wrong_token() {
    let mut deps = setup_test();

    let provide_msg = ExecuteMsg::ProvideLiquidity {
        asset: native_asset("notsupported".into(), Uint128::new(100_000000)),
        receiver: None,
    };

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("user", &[coin(100_000000, "notsupported")]),
        provide_msg,
    );

    assert_eq!(res, Err(ContractError::AssetMismatch {}))
}

#[test]
fn provide_liquidity_wrong_amount() {
    let mut deps = setup_test();

    let provide_msg = ExecuteMsg::ProvideLiquidity {
        asset: native_asset("utoken".into(), Uint128::new(123_000000)),
        receiver: None,
    };

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("user", &[coin(100_000000, "utoken")]),
        provide_msg,
    )
    .unwrap_err();

    assert_eq!(
        res.to_string(),
        "Generic error: Native token balance mismatch between the argument and the transferred"
            .to_string()
    )
}

#[test]
fn provide_liquidity_zero_throws() {
    let mut deps = setup_test();

    let provide_msg = ExecuteMsg::ProvideLiquidity {
        asset: native_asset("utoken".into(), Uint128::new(0)),
        receiver: None,
    };

    let res =
        execute(deps.as_mut(), mock_env(), mock_info("user", &[coin(0, "utoken")]), provide_msg)
            .unwrap_err();

    assert_eq!(res, ContractError::InvalidZeroAmount {})
}

fn _provide_liquidity() -> (OwnedDeps<MockStorage, MockApi, CustomQuerier>, Response) {
    let mut deps = setup_test();

    // pre apply utoken amount
    deps.querier.set_bank_balance(100_000000);
    deps.querier.set_cw20_total_supply("lptoken", 0);
    // this is used to fake calculating the share.
    deps.querier.set_cw20_balance("lptoken", "share_user", 50_000000u128);

    let provide_msg = ExecuteMsg::ProvideLiquidity {
        asset: native_asset("utoken".to_string(), Uint128::new(100_000000)),
        receiver: None,
    };

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("user", &[coin(100_000000, "utoken")]),
        provide_msg,
    )
    .unwrap();

    deps.querier.set_cw20_total_supply("lptoken", 100_000000);
    deps.querier.set_cw20_balance("lptoken", "user", 100_000000);

    (deps, res)
}

#[test]
fn provide_liquidity_success() {
    let (_deps, res) = _provide_liquidity();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "arb/provide_liquidity"),
            attr("sender", "user"),
            attr("recipient", "user"),
            attr("vault_utoken_before", "0"),
            attr("vault_utoken_after", "100000000"),
            attr("share", "100000000")
        ]
    );
}

fn _provide_liquidity_again() -> (OwnedDeps<MockStorage, MockApi, CustomQuerier>, Response) {
    let (mut deps, _res) = _provide_liquidity();

    deps.querier.set_bank_balance(100_000000 + 120_000000);

    let provide_msg = ExecuteMsg::ProvideLiquidity {
        asset: native_asset("utoken".to_string(), Uint128::new(120_000000)),
        receiver: None,
    };

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("user", &[coin(120_000000, "utoken")]),
        provide_msg,
    )
    .unwrap();

    deps.querier.set_cw20_total_supply("lptoken", 220_000000);
    deps.querier.set_cw20_balance("lptoken", "user", 220_000000);

    (deps, res)
}

#[test]
fn provide_liquidity_again_success() {
    let (_deps, res) = _provide_liquidity_again();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "arb/provide_liquidity"),
            attr("sender", "user"),
            attr("recipient", "user"),
            attr("vault_utoken_before", "100000000"),
            attr("vault_utoken_after", "220000000"),
            attr("share", "120000000")
        ]
    );
}

#[test]
fn query_user_info_check() {
    let (mut deps, _res) = _provide_liquidity_again();

    let response = query_user_info(deps.as_ref(), mock_env(), "user".to_string()).unwrap();
    assert_eq!(
        response,
        UserInfoResponse {
            utoken_amount: Uint128::new(220_000000),
            lp_amount: Uint128::new(220_000000),
        }
    );

    // arbs executed and created 2 luna
    deps.querier.set_bank_balances(&[coin(222_000000, "utoken")]);

    let response = query_user_info(deps.as_ref(), mock_env(), "user".to_string()).unwrap();
    assert_eq!(
        response,
        UserInfoResponse {
            utoken_amount: Uint128::new(222_000000),
            lp_amount: Uint128::new(220_000000),
        }
    );

    /* through arbs, 3 more luna are currently unbonding were generated */
    deps.querier.with_unbonding(Uint128::new(3_000000u128));

    let response = query_user_info(deps.as_ref(), mock_env(), "user".to_string()).unwrap();

    let stader_unbonding = Decimal::from_ratio(102u128, 100u128) * Uint128::new(3_000000u128);
    let steak_unbonding = Uint128::new(3_000000u128);
    let eris_unbonding = Decimal::from_str("1.1").unwrap() * Uint128::new(3_000000u128);
    let prism_unbonding = Uint128::new(3_000000u128);

    assert_eq!(
        response,
        UserInfoResponse {
            utoken_amount: Uint128::new(222_000000)
                + stader_unbonding
                + steak_unbonding
                + eris_unbonding
                + prism_unbonding,
            lp_amount: Uint128::new(220_000000),
        }
    );

    /* through arbs, 4 more luna can currently be claimed */
    deps.querier.with_withdrawable(Uint128::new(4_000000u128));

    let stader_withdrawing = Decimal::from_ratio(102u128, 100u128) * Uint128::new(4_000000u128);
    let steak_withdrawing = Uint128::new(4_000000u128);
    let eris_withdrawing = Decimal::from_str("1.1").unwrap() * Uint128::new(4_000000u128);
    let prism_withdrawing = Uint128::new(4_000000u128);

    let response = query_user_info(deps.as_ref(), mock_env(), "user".to_string()).unwrap();
    assert_eq!(
        response,
        UserInfoResponse {
            utoken_amount: Uint128::new(222_000000)
                + stader_unbonding
                + steak_unbonding
                + eris_unbonding
                + prism_unbonding
                + stader_withdrawing
                + steak_withdrawing
                + eris_withdrawing
                + prism_withdrawing,
            lp_amount: Uint128::new(220_000000),
        }
    );
}

#[test]
fn throws_if_provided_profit_not_found() {
    let mut deps = setup_test();

    let whitelist_info = mock_info("whitelisted_exec", &[]);

    let exec_msg = ExecuteMsg::ExecuteArbitrage {
        msg: ExecuteSubMsg {
            contract_addr: None,
            msg: to_binary(&Empty {}).unwrap(),
            funds_amount: Uint128::new(100_000000u128),
        },
        result_token: token_asset_info(Addr::unchecked("eriscw")),
        wanted_profit: Decimal::from_ratio(10u128, 100u128),
    };

    let result = execute(deps.as_mut(), mock_env(), whitelist_info, exec_msg).unwrap_err();

    assert_eq!(result, ContractError::NotSupportedProfitStep(Decimal::from_str("0.1").unwrap()));
}

#[test]
fn throws_if_not_whitelisted_executor() {
    let mut deps = setup_test();

    let user_info = mock_info("user", &[]);
    let whitelist_info = mock_info("whitelisted_exec", &[]);

    let execute_msg = ExecuteMsg::ExecuteArbitrage {
        msg: ExecuteSubMsg {
            contract_addr: None,
            msg: to_binary(&Empty {}).unwrap(),
            funds_amount: Uint128::new(100_000000u128),
        },
        result_token: token_asset_info(Addr::unchecked("eriscw")),
        wanted_profit: Decimal::from_ratio(1u128, 100u128),
    };

    let withdraw_msg = ExecuteMsg::WithdrawFromLiquidStaking {};

    //
    // NOT WHITELISTED
    //
    let result =
        execute(deps.as_mut(), mock_env(), user_info.clone(), execute_msg.clone()).unwrap_err();
    assert_eq!(result, ContractError::UnauthorizedNotWhitelisted {});

    let result = execute(deps.as_mut(), mock_env(), user_info, withdraw_msg.clone()).unwrap_err();
    assert_eq!(result, ContractError::UnauthorizedNotWhitelisted {});

    //
    // WHITELISTED
    //
    let result =
        execute(deps.as_mut(), mock_env(), whitelist_info.clone(), execute_msg).unwrap_err();

    assert_eq!(result, ContractError::NotEnoughFundsTakeable {});

    let result = execute(deps.as_mut(), mock_env(), whitelist_info, withdraw_msg).unwrap_err();
    assert_eq!(result, ContractError::NothingToWithdraw {});
}

#[test]
fn throws_if_has_withdraw() {
    let mut deps = setup_test();

    let whitelist_info = mock_info("whitelisted_exec", &[]);

    let withdraw_msg = ExecuteMsg::WithdrawFromLiquidStaking {};

    let result =
        execute(deps.as_mut(), mock_env(), whitelist_info.clone(), withdraw_msg).unwrap_err();
    assert_eq!(result, ContractError::NothingToWithdraw {});

    deps.querier.with_withdrawable(Uint128::new(10));
    deps.querier.set_bank_balances(&[coin(222_000000, "utoken")]);

    let execute_msg = ExecuteMsg::ExecuteArbitrage {
        msg: ExecuteSubMsg {
            contract_addr: None,
            msg: to_binary(&Empty {}).unwrap(),
            funds_amount: Uint128::new(100_000000u128),
        },
        result_token: token_asset_info(Addr::unchecked("eriscw")),
        wanted_profit: Decimal::from_ratio(1u128, 100u128),
    };
    let result = execute(deps.as_mut(), mock_env(), whitelist_info, execute_msg).unwrap_err();

    assert_eq!(result, ContractError::WithdrawBeforeExecute {});
}

#[test]
fn check_withdrawing() {
    let mut deps = setup_test();

    let whitelist_info = mock_info("whitelisted_exec", &[]);

    let withdraw_msg = ExecuteMsg::WithdrawFromLiquidStaking {};

    deps.querier.with_withdrawable(Uint128::new(10_000000u128));

    let result = execute(deps.as_mut(), mock_env(), whitelist_info, withdraw_msg)
        .expect("expected response");

    assert_eq!(
        result.attributes,
        vec![
            attr("action", "arb/execute_withdraw_liquidity"),
            attr("type", "eris"), // eris has factor 1.1
            attr("withdraw_amount", "11000000"),
            attr("type", "steak"),
            attr("withdraw_amount", "10000000"),
            attr("type", "stader"), // stader has factor 1.02
            attr("withdraw_amount", "10200000"),
            attr("type", "prism"),
            attr("withdraw_amount", "10000000"),
        ]
    );

    // eris + backbone + stader + prism
    assert_eq!(result.messages.len(), 4);

    // eris
    match result.messages[0].msg.clone() {
        CosmosMsg::Wasm(WasmMsg::Execute {
            funds,
            contract_addr,
            msg,
        }) => {
            assert_eq!(contract_addr, "eris".to_string());
            assert_eq!(funds.len(), 0);
            let sub_msg: eris::hub::ExecuteMsg = from_binary(&msg).unwrap();

            assert_eq!(
                sub_msg,
                eris::hub::ExecuteMsg::WithdrawUnbonded {
                    receiver: None
                }
            );
        },
        _ => panic!("DO NOT ENTER HERE"),
    }

    // backbone
    match result.messages[1].msg.clone() {
        CosmosMsg::Wasm(WasmMsg::Execute {
            funds,
            contract_addr,
            msg,
        }) => {
            assert_eq!(contract_addr, "backbone".to_string());
            assert_eq!(funds.len(), 0);
            let sub_msg: steak::hub::ExecuteMsg = from_binary(&msg).unwrap();

            assert_eq!(
                sub_msg,
                steak::hub::ExecuteMsg::WithdrawUnbonded {
                    receiver: None
                }
            );
        },
        _ => panic!("DO NOT ENTER HERE"),
    }

    // stader
    match result.messages[2].msg.clone() {
        CosmosMsg::Wasm(WasmMsg::Execute {
            funds,
            contract_addr,
            msg,
        }) => {
            assert_eq!(contract_addr, "stader");
            assert_eq!(funds.len(), 0);
            let sub_msg: stader::msg::ExecuteMsg = from_binary(&msg).unwrap();

            assert_eq!(
                sub_msg,
                stader::msg::ExecuteMsg::WithdrawFundsToWallet {
                    batch_id: 0u64
                }
            );
        },
        _ => panic!("DO NOT ENTER HERE"),
    }

    // prism
    match result.messages[3].msg.clone() {
        CosmosMsg::Wasm(WasmMsg::Execute {
            funds,
            contract_addr,
            msg,
        }) => {
            assert_eq!(contract_addr, "prism");
            assert_eq!(funds.len(), 0);
            let sub_msg: prism::hub::ExecuteMsg = from_binary(&msg).unwrap();

            assert_eq!(
                format!("{:?}", sub_msg),
                format!("{:?}", prism::hub::ExecuteMsg::WithdrawUnbonded {})
            )
        },
        _ => panic!("DO NOT ENTER HERE"),
    }
}

fn _unbonding_slow_120() -> (OwnedDeps<MockStorage, MockApi, CustomQuerier>, Response) {
    // deposit 100
    // deposit 120
    // withdraw 120

    let (mut deps, _res) = _provide_liquidity_again();

    let lptoken_cw20 = mock_info("lptoken", &[]);

    let withdraw = ExecuteMsg::Receive(Cw20ReceiveMsg {
        amount: Uint128::new(120_000000u128),
        sender: "user001".to_string(),
        msg: to_binary(&Cw20HookMsg::Unbond {
            immediate: Some(false),
        })
        .unwrap(),
    });

    let res = execute(deps.as_mut(), mock_env(), lptoken_cw20, withdraw).unwrap();

    deps.querier.set_bank_balances(&[coin(220_000000u128, "utoken")]);
    deps.querier.set_cw20_total_supply("lptoken", 100_000000);

    // deps.querier.with_token_balances(&[(
    //     &String::from("lptoken"),
    //     &[(&String::from(MOCK_CONTRACT_ADDR), &Uint128::new(100_000000u128))],
    // )]);

    (deps, res)
}

#[test]
fn withdrawing_liquidity_success() {
    let (deps, res) = _unbonding_slow_120();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "arb/execute_unbond"),
            attr("from", "user001"),
            attr("withdraw_amount", "120000000"),
            attr("receive_amount", "117600000"),
            attr("protocol_fee", "2400000"),
            attr("vault_total", "220000000"),
            attr("total_supply", "220000000"),
            attr("unbond_time_s", "100"),
            attr("burnt_amount", "120000000")
        ]
    );

    // withdraw + fee
    assert_eq!(res.messages.len(), 1);

    match res.messages[0].msg.clone() {
        CosmosMsg::Wasm(WasmMsg::Execute {
            funds,
            contract_addr,
            msg,
        }) => {
            assert_eq!(contract_addr, "lptoken".to_string());
            assert_eq!(funds.len(), 0);
            let sub_msg: Cw20ExecuteMsg = from_binary(&msg).unwrap();

            assert_eq!(
                sub_msg,
                Cw20ExecuteMsg::Burn {
                    amount: Uint128::new(120_000000u128)
                }
            );
        },
        _ => panic!("DO NOT ENTER HERE"),
    }

    // check unbonding history correct start
    let unbonding = query_unbond_requests(
        deps.as_ref(),
        _mock_env_at_timestamp(1),
        "user001".to_string(),
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        unbonding,
        UnbondRequestsResponse {
            requests: vec![UnbondItem {
                start_time: 1,
                release_time: 1 + 100,
                amount_asset: Uint128::new(120000000),
                id: 0,
                withdraw_protocol_fee: Uint128::new(2400000),
                // 0.05 * 120000000 = 6000000
                withdraw_pool_fee: Uint128::new(6000000),
                released: false,
            }]
        }
    );

    // check unbonding history correct in the middle
    let unbonding = query_unbond_requests(
        deps.as_ref(),
        _mock_env_at_timestamp(10),
        "user001".to_string(),
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        unbonding,
        UnbondRequestsResponse {
            requests: vec![UnbondItem {
                start_time: 1,
                release_time: 1 + 100,
                amount_asset: Uint128::new(120000000),
                id: 0,
                withdraw_protocol_fee: Uint128::new(2400000),
                withdraw_pool_fee: Uint128::new(5460000),
                released: false,
            }]
        }
    );

    // check unbonding history correct after release
    let unbonding = query_unbond_requests(
        deps.as_ref(),
        _mock_env_at_timestamp(101),
        "user001".to_string(),
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        unbonding,
        UnbondRequestsResponse {
            requests: vec![UnbondItem {
                start_time: 1,
                release_time: 1 + 100,
                amount_asset: Uint128::new(120000000),
                id: 0,
                withdraw_protocol_fee: Uint128::new(2400000),
                withdraw_pool_fee: Uint128::new(0),
                released: true,
            }]
        }
    );
}

fn _unbonding_slow_with_pool_unbonding(
) -> (OwnedDeps<MockStorage, MockApi, CustomQuerier>, Response) {
    let (mut deps, _res) = _provide_liquidity_again();

    // arbs executed and created 2 luna
    deps.querier.set_bank_balance(100_000000);
    deps.querier.with_unbonding(Uint128::new(24_000000u128));

    let lptoken_cw20 = mock_info("lptoken", &[]);

    let unbond = ExecuteMsg::Receive(Cw20ReceiveMsg {
        amount: Uint128::new(120_000000u128),
        sender: "user001".to_string(),
        msg: to_binary(&Cw20HookMsg::Unbond {
            immediate: Some(false),
        })
        .unwrap(),
    });

    let res = execute(deps.as_mut(), mock_env(), lptoken_cw20, unbond).unwrap();

    deps.querier.set_cw20_total_supply("lptoken", 100_000000);
    deps.querier.set_cw20_balance("lptoken", "user", 120_000000);

    (deps, res)
}

fn get_unbonding_value(set: u128) -> Uint128 {
    let set = Uint128::new(set);
    let eris_unbonding = Decimal::from_str("1.1").unwrap() * set;
    let prism_unbonding = set;
    let stader_unbonding = Decimal::from_ratio(102u128, 100u128) * set;
    let steak_unbonding = set;

    prism_unbonding + stader_unbonding + eris_unbonding + steak_unbonding
}
fn get_withdraw_value(set: u128) -> Uint128 {
    let set = Uint128::new(set);
    let prism = set;
    let eris = Decimal::from_str("1.1").unwrap() * set;
    let stader = Decimal::from_ratio(102u128, 100u128) * set;
    let steak = set;

    prism + stader + eris + steak
}

#[test]
fn withdrawing_liquidity_with_unbonding_success() {
    let (_deps, res) = _unbonding_slow_with_pool_unbonding();

    let pool_value = Uint128::new(100_000000u128) + get_unbonding_value(24_000000u128);
    let expected_asset = pool_value.multiply_ratio(120u128, 220u128);
    let fee = Decimal::from_str("0.02").unwrap() * expected_asset;
    let receive = expected_asset - fee;

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "arb/execute_unbond"),
            attr("from", "user001"),
            attr("withdraw_amount", expected_asset),
            attr("receive_amount", receive),
            attr("protocol_fee", fee),
            attr("vault_total", pool_value),
            attr("total_supply", "220000000"),
            attr("unbond_time_s", "100"),
            attr("burnt_amount", "120000000")
        ]
    );

    // withdraw + fee
    assert_eq!(res.messages.len(), 1);

    match res.messages[0].msg.clone() {
        CosmosMsg::Wasm(WasmMsg::Execute {
            funds,
            contract_addr,
            msg,
        }) => {
            assert_eq!(contract_addr, "lptoken".to_string());
            assert_eq!(funds.len(), 0);
            let sub_msg: Cw20ExecuteMsg = from_binary(&msg).unwrap();

            assert_eq!(
                sub_msg,
                Cw20ExecuteMsg::Burn {
                    amount: Uint128::new(120_000000u128)
                }
            );
        },
        _ => panic!("DO NOT ENTER HERE"),
    }
}

#[test]
fn withdraw_liquidity_immediate_user_unbonding_no_liquidity_throws() {
    let (mut deps, _res) = _unbonding_slow_with_pool_unbonding();

    let lptoken_cw20 = mock_info("lptoken", &[]);

    let withdraw = ExecuteMsg::Receive(Cw20ReceiveMsg {
        amount: Uint128::new(100_000000u128),
        sender: "user001".to_string(),
        msg: to_binary(&Cw20HookMsg::Unbond {
            immediate: Some(true),
        })
        .unwrap(),
    });

    let result = execute(deps.as_mut(), mock_env(), lptoken_cw20, withdraw).unwrap_err();

    // withdraw + fee
    assert_eq!(result, ContractError::NotEnoughAssetsInThePool {});
}

#[test]
fn withdraw_liquidity_immediate_tokens_unbonding_no_liquidity_throws() {
    let (mut deps, _res) = _provide_liquidity_again();

    deps.querier.set_bank_balance(100_000000);

    // is some factor of 120 LUNA unbonding + some rewards = 4*24
    deps.querier.with_unbonding(Uint128::new(24_000000u128));

    let lptoken_cw20 = mock_info("lptoken", &[]);

    let withdraw = ExecuteMsg::Receive(Cw20ReceiveMsg {
        amount: Uint128::new(120_000000u128),
        sender: "user001".to_string(),
        msg: to_binary(&Cw20HookMsg::Unbond {
            immediate: Some(true),
        })
        .unwrap(),
    });

    let result =
        execute(deps.as_mut(), mock_env(), lptoken_cw20, withdraw).expect_err("expected an error");

    // withdraw + fee
    assert_eq!(result, ContractError::NotEnoughAssetsInThePool {});
}

#[test]
fn withdraw_liquidity_immediate_success() {
    let (mut deps, _res) = _provide_liquidity_again();

    // total_asset: 220
    // pool made 2 through arbs
    let total_pool = Uint128::new(100_000000u128 + 120_000000u128 + 2_000000u128);

    // arbs executed and created 2 luna
    deps.querier.set_bank_balance(222_000000);

    let lptoken_cw20 = mock_info("lptoken", &[]);
    let user = mock_info("user001", &[]);

    let withdraw = ExecuteMsg::Receive(Cw20ReceiveMsg {
        amount: Uint128::new(100_000000u128),
        sender: "user001".to_string(),
        msg: to_binary(&Cw20HookMsg::Unbond {
            immediate: Some(true),
        })
        .unwrap(),
    });

    let result = execute(deps.as_mut(), mock_env(), user, withdraw.clone()).unwrap_err();
    assert_eq!(result, ContractError::ExpectingLPToken("user001".to_string()));

    let result =
        execute(deps.as_mut(), mock_env(), lptoken_cw20, withdraw).expect("expected a result");

    let withdraw_pool_amount = Decimal::from_ratio(100u128, 220u128) * total_pool;
    let pool_fee = Decimal::from_str("0.05").unwrap() * withdraw_pool_amount;
    let protocol_fee = Decimal::from_str("0.02").unwrap() * withdraw_pool_amount;
    assert_eq!(
        result.attributes,
        vec![
            attr("action", "arb/execute_withdraw"),
            attr("from", MOCK_CONTRACT_ADDR),
            attr("receiver", "user001"),
            attr("withdraw_amount", withdraw_pool_amount),
            attr("receive_amount", withdraw_pool_amount - pool_fee - protocol_fee),
            attr("protocol_fee", protocol_fee),
            attr("pool_fee", pool_fee),
            attr("immediate", true.to_string()),
            attr("burnt_amount", "100000000")
        ]
    );

    // withdraw + fee + burn
    assert_eq!(result.messages.len(), 3);

    match result.messages[0].msg.clone() {
        CosmosMsg::Bank(BankMsg::Send {
            to_address,
            amount,
        }) => {
            assert_eq!(to_address, "user001".to_string());
            assert_eq!(amount.len(), 1);
            assert_eq!(
                amount[0],
                Coin {
                    denom: "utoken".to_string(),
                    amount: withdraw_pool_amount - pool_fee - protocol_fee
                }
            );
        },

        _ => panic!("DO NOT ENTER HERE"),
    }

    match result.messages[1].msg.clone() {
        CosmosMsg::Bank(BankMsg::Send {
            to_address,
            amount,
        }) => {
            assert_eq!(to_address, "fee".to_string());
            assert_eq!(amount.len(), 1);
            assert_eq!(
                amount[0],
                Coin {
                    denom: "utoken".to_string(),
                    amount: protocol_fee
                }
            );
        },

        _ => panic!("DO NOT ENTER HERE"),
    }

    match result.messages[2].msg.clone() {
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            funds,
            msg,
        }) => {
            assert_eq!(contract_addr, "lptoken".to_string());
            assert_eq!(funds.len(), 0);

            let sub_msg: Cw20ExecuteMsg = from_binary(&msg).unwrap();

            assert_eq!(
                sub_msg,
                Cw20ExecuteMsg::Burn {
                    amount: Uint128::new(100_000000u128)
                }
            );
        },

        _ => panic!("DO NOT ENTER HERE"),
    }
}

#[test]
fn withdraw_liquidity_unbonding_query_requests_success() {
    let (mut deps, _res) = _unbonding_slow_120();

    //
    // UNBONDING AGAIN WITH OTHER TIME
    //

    let lptoken_cw20 = mock_info("lptoken", &[]);
    let user = mock_info("user001", &[]);
    let mid_time = _mock_env_at_timestamp(51);
    let end_time = _mock_env_at_timestamp(200);

    let unbonding_again = ExecuteMsg::Receive(Cw20ReceiveMsg {
        amount: Uint128::new(10_000000u128),
        sender: "user001".to_string(),
        msg: to_binary(&Cw20HookMsg::Unbond {
            immediate: Some(false),
        })
        .unwrap(),
    });

    let res = execute(deps.as_mut(), mid_time.clone(), lptoken_cw20, unbonding_again).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "arb/execute_unbond"),
            attr("from", "user001"),
            attr("withdraw_amount", "10000000"),
            attr("receive_amount", "9800000"),
            attr("protocol_fee", "200000"),
            attr("vault_total", "100000000"),
            attr("total_supply", "100000000"),
            attr("unbond_time_s", "100"),
            attr("burnt_amount", "10000000")
        ]
    );

    let unbonding =
        query_unbond_requests(deps.as_ref(), mid_time.clone(), "user001".to_string(), None, None)
            .unwrap();

    assert_eq!(
        unbonding,
        UnbondRequestsResponse {
            requests: vec![
                UnbondItem {
                    start_time: 1,
                    release_time: 1 + 100,
                    amount_asset: Uint128::new(120_000000u128),
                    id: 0,
                    withdraw_protocol_fee: Uint128::new(2400000),
                    withdraw_pool_fee: Uint128::new(3000000),
                    released: false
                },
                UnbondItem {
                    start_time: 51,
                    release_time: 51 + 100,
                    amount_asset: Uint128::new(10_000000u128),
                    id: 1,
                    withdraw_protocol_fee: Uint128::new(200000),
                    withdraw_pool_fee: Uint128::new(500000),
                    released: false,
                }
            ]
        },
    );

    let share = query_utoken(deps.as_ref());
    //
    // WITHDRAW IMMEDIATE
    //
    let withdraw_immediate = ExecuteMsg::WithdrawImmediate {
        id: 0,
    };

    let res = execute(deps.as_mut(), mid_time.clone(), user.clone(), withdraw_immediate).unwrap();

    let withdraw_pool_amount = Uint128::new(120_000000u128);
    let pool_fee = Decimal::from_str("0.05").unwrap()
        * withdraw_pool_amount
        * Decimal::from_str("0.5").unwrap();
    let protocol_fee = Decimal::from_str("0.02").unwrap() * withdraw_pool_amount;
    let receive_amount = withdraw_pool_amount - pool_fee - protocol_fee;

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "arb/execute_withdraw"),
            attr("from", "cosmos2contract"),
            attr("receiver", "user001"),
            attr("withdraw_amount", withdraw_pool_amount),
            attr("receive_amount", receive_amount),
            attr("protocol_fee", protocol_fee),
            attr("pool_fee", pool_fee),
            attr("immediate", true.to_string()),
        ]
    );

    // withdraw + fee (without burn)
    assert_eq!(res.messages.len(), 2);

    match res.messages[0].msg.clone() {
        CosmosMsg::Bank(BankMsg::Send {
            to_address,
            amount,
        }) => {
            assert_eq!(to_address, "user001".to_string());
            assert_eq!(amount.len(), 1);
            assert_eq!(
                amount[0],
                Coin {
                    denom: "utoken".to_string(),
                    amount: receive_amount
                }
            );
        },

        _ => panic!("DO NOT ENTER HERE"),
    }

    match res.messages[1].msg.clone() {
        CosmosMsg::Bank(BankMsg::Send {
            to_address,
            amount,
        }) => {
            assert_eq!(to_address, "fee".to_string());
            assert_eq!(amount.len(), 1);
            assert_eq!(
                amount[0],
                Coin {
                    denom: "utoken".to_string(),
                    amount: protocol_fee
                }
            );
        },

        _ => panic!("DO NOT ENTER HERE"),
    }

    let unbonding =
        query_unbond_requests(deps.as_ref(), mid_time, "user001".to_string(), None, None).unwrap();

    assert_eq!(
        unbonding,
        UnbondRequestsResponse {
            requests: vec![UnbondItem {
                start_time: 51,
                release_time: 51 + 100,
                amount_asset: Uint128::new(10_000000u128),
                id: 1,
                withdraw_protocol_fee: Uint128::new(200000),
                withdraw_pool_fee: Uint128::new(500000),
                released: false
            }]
        }
    );

    deps.querier.set_bank_balance(220_000000 - receive_amount.u128() - protocol_fee.u128());

    // share value is increased by the half protocol fee (share is 50 / 100)
    let share2 = query_utoken(deps.as_ref());
    assert_eq!(share + pool_fee * Decimal::from_str("0.5").unwrap(), share2);

    //
    // WITHDRAW IMMEDIATE AFTER END
    //
    let unbonding =
        query_unbond_requests(deps.as_ref(), end_time.clone(), "user001".to_string(), None, None)
            .unwrap();

    assert_eq!(
        unbonding,
        UnbondRequestsResponse {
            requests: vec![UnbondItem {
                start_time: 51,
                release_time: 51 + 100,
                amount_asset: Uint128::new(10_000000u128),
                id: 1,
                withdraw_protocol_fee: Uint128::new(200000),
                withdraw_pool_fee: Uint128::new(0u128),
                released: true
            }]
        }
    );

    let withdraw_immediate = ExecuteMsg::WithdrawImmediate {
        id: 1,
    };

    let res = execute(deps.as_mut(), end_time.clone(), user, withdraw_immediate).unwrap();

    let withdraw_pool_amount = Uint128::new(10_000000u128);
    let pool_fee2 = Uint128::zero();
    let protocol_fee2 = Decimal::from_str("0.02").unwrap() * withdraw_pool_amount;
    let receive_amount2 = withdraw_pool_amount - pool_fee2 - protocol_fee2;

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "arb/execute_withdraw"),
            attr("from", "cosmos2contract"),
            attr("receiver", "user001"),
            attr("withdraw_amount", withdraw_pool_amount),
            attr("receive_amount", receive_amount2),
            attr("protocol_fee", protocol_fee2),
            attr("pool_fee", pool_fee2),
            attr("immediate", false.to_string()),
        ]
    );

    // withdraw + fee (without burn)
    assert_eq!(res.messages.len(), 2);

    match res.messages[0].msg.clone() {
        CosmosMsg::Bank(BankMsg::Send {
            to_address,
            amount,
        }) => {
            assert_eq!(to_address, "user001".to_string());
            assert_eq!(amount.len(), 1);
            assert_eq!(
                amount[0],
                Coin {
                    denom: "utoken".to_string(),
                    amount: receive_amount2
                }
            );
        },

        _ => panic!("DO NOT ENTER HERE"),
    }

    match res.messages[1].msg.clone() {
        CosmosMsg::Bank(BankMsg::Send {
            to_address,
            amount,
        }) => {
            assert_eq!(to_address, "fee".to_string());
            assert_eq!(amount.len(), 1);
            assert_eq!(
                amount[0],
                Coin {
                    denom: "utoken".to_string(),
                    amount: protocol_fee2
                }
            );
        },

        _ => panic!("DO NOT ENTER HERE"),
    }

    let unbonding =
        query_unbond_requests(deps.as_ref(), end_time, "user001".to_string(), None, None).unwrap();

    assert_eq!(
        unbonding,
        UnbondRequestsResponse {
            requests: vec![],
        }
    );

    deps.querier.set_bank_balance(
        220_000000u128
            - receive_amount.u128()
            - protocol_fee.u128()
            - receive_amount2.u128()
            - protocol_fee2.u128(),
    );

    let share3 = query_utoken(deps.as_ref());
    // share is not allowed to change by withdrawing after the end time
    assert_eq!(share2, share3);
}

#[test]
fn withdraw_liquidity_unbonded_all_success() {
    let (mut deps, _res) = _unbonding_slow_120();

    //
    // UNBONDING AGAIN WITH OTHER TIME
    //

    let lptoken_cw20 = mock_info("lptoken", &[]);
    let user = mock_info("user001", &[]);
    let mid_time = _mock_env_at_timestamp(51);
    let end_time = _mock_env_at_timestamp(200);

    let unbonding_again = ExecuteMsg::Receive(Cw20ReceiveMsg {
        amount: Uint128::new(10_000000u128),
        sender: "user001".to_string(),
        msg: to_binary(&Cw20HookMsg::Unbond {
            immediate: Some(false),
        })
        .unwrap(),
    });

    let _res = execute(deps.as_mut(), mid_time.clone(), lptoken_cw20, unbonding_again).unwrap();

    let unbonding =
        query_unbond_requests(deps.as_ref(), end_time.clone(), "user001".to_string(), None, None)
            .unwrap();

    assert_eq!(
        unbonding,
        UnbondRequestsResponse {
            requests: vec![
                UnbondItem {
                    start_time: 1,
                    release_time: 1 + 100,
                    amount_asset: Uint128::new(120_000000u128),
                    id: 0,
                    withdraw_protocol_fee: Uint128::new(2400000),
                    withdraw_pool_fee: Uint128::new(0_000000u128),
                    released: true
                },
                UnbondItem {
                    start_time: 51,
                    release_time: 51 + 100,
                    amount_asset: Uint128::new(10_000000u128),
                    id: 1,
                    withdraw_protocol_fee: Uint128::new(200000u128),
                    withdraw_pool_fee: Uint128::new(0_000000u128),
                    released: true
                }
            ]
        }
    );

    deps.querier.set_cw20_balance("lptoken", "share_user", 50_000000);
    let share = query_utoken(deps.as_ref());

    //
    // WITHDRAW UNBONDED FAILED
    //
    let withdraw_unbonded = ExecuteMsg::WithdrawUnbonded {};

    let res =
        execute(deps.as_mut(), mid_time, user.clone(), withdraw_unbonded.clone()).unwrap_err();

    assert_eq!(res, ContractError::NoWithdrawableAsset {});

    //
    // WITHDRAW UNBONDED
    //
    let res = execute(deps.as_mut(), end_time.clone(), user.clone(), withdraw_unbonded)
        .expect("expect response");

    let withdraw_pool_amount = Uint128::from(130_000000u128);
    let pool_fee = Uint128::zero();
    let protocol_fee = Decimal::from_str("0.02").unwrap() * withdraw_pool_amount;
    let receive_amount = withdraw_pool_amount - pool_fee - protocol_fee;

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "arb/execute_withdraw"),
            attr("from", "cosmos2contract"),
            attr("receiver", "user001"),
            attr("withdraw_amount", withdraw_pool_amount),
            attr("receive_amount", receive_amount),
            attr("protocol_fee", protocol_fee),
            attr("pool_fee", pool_fee),
            attr("immediate", false.to_string()),
            // no burn, as it already happend during normal withdraw
            // attr("burnt_amount", "100000000")
        ]
    );

    // withdraw + fee (without burn)
    assert_eq!(res.messages.len(), 2);

    match res.messages[0].msg.clone() {
        CosmosMsg::Bank(BankMsg::Send {
            to_address,
            amount,
        }) => {
            assert_eq!(to_address, "user001".to_string());
            assert_eq!(amount.len(), 1);
            assert_eq!(
                amount[0],
                Coin {
                    denom: "utoken".to_string(),
                    amount: receive_amount
                }
            );
        },

        _ => panic!("DO NOT ENTER HERE"),
    }

    match res.messages[1].msg.clone() {
        CosmosMsg::Bank(BankMsg::Send {
            to_address,
            amount,
        }) => {
            assert_eq!(to_address, "fee".to_string());
            assert_eq!(amount.len(), 1);
            assert_eq!(
                amount[0],
                Coin {
                    denom: "utoken".to_string(),
                    amount: protocol_fee
                }
            );
        },

        _ => panic!("DO NOT ENTER HERE"),
    }

    deps.querier.set_bank_balance(220_000000u128 - receive_amount.u128() - protocol_fee.u128());

    // share value is not changed, as there is no pool fee
    let share2 = query_utoken(deps.as_ref());
    assert_eq!(share, share2);

    let unbonding =
        query_unbond_requests(deps.as_ref(), end_time.clone(), "user001".to_string(), None, None)
            .unwrap();

    // no items
    assert_eq!(
        unbonding,
        UnbondRequestsResponse {
            requests: vec![]
        }
    );

    //
    // WITHDRAW UNBONDED FAILED
    //
    let withdraw_unbonded = ExecuteMsg::WithdrawUnbonded {};

    let res = execute(deps.as_mut(), end_time, user, withdraw_unbonded).unwrap_err();

    assert_eq!(res, ContractError::NoWithdrawableAsset {});
}

#[test]
fn withdraw_liquidity_unbonded_half_success() {
    let (mut deps, _res) = _unbonding_slow_120();

    // difference is that we only unbond part of the history instead of everything
    //
    // UNBONDING AGAIN WITH OTHER TIME
    //

    let lptoken_cw20 = mock_info("lptoken", &[]);
    let user = mock_info("user001", &[]);
    let mid_time = _mock_env_at_timestamp(51);
    let before_end_time = _mock_env_at_timestamp(130);
    let end_time = _mock_env_at_timestamp(200);

    let unbonding_again = ExecuteMsg::Receive(Cw20ReceiveMsg {
        amount: Uint128::new(10_000000u128),
        sender: "user001".to_string(),
        msg: to_binary(&Cw20HookMsg::Unbond {
            immediate: Some(false),
        })
        .unwrap(),
    });

    let _res = execute(deps.as_mut(), mid_time, lptoken_cw20, unbonding_again).unwrap();

    let unbonding =
        query_unbond_requests(deps.as_ref(), end_time.clone(), "user001".to_string(), None, None)
            .unwrap();

    assert_eq!(
        unbonding,
        UnbondRequestsResponse {
            requests: vec![
                UnbondItem {
                    start_time: 1,
                    release_time: 1 + 100,
                    amount_asset: Uint128::new(120_000000u128),
                    id: 0,
                    withdraw_protocol_fee: Uint128::new(2400000),
                    withdraw_pool_fee: Uint128::new(0u128),
                    released: true,
                },
                UnbondItem {
                    start_time: 51,
                    release_time: 51 + 100,
                    amount_asset: Uint128::new(10_000000u128),
                    id: 1,
                    withdraw_protocol_fee: Uint128::new(200000),
                    withdraw_pool_fee: Uint128::new(0u128),
                    released: true,
                }
            ],
        }
    );

    deps.querier.set_cw20_balance("lptoken", "share_user", 50_000000);
    let share = query_utoken(deps.as_ref());

    //
    // WITHDRAW UNBONDED
    //
    let withdraw_unbonded = ExecuteMsg::WithdrawUnbonded {};
    let res = execute(deps.as_mut(), before_end_time.clone(), user.clone(), withdraw_unbonded)
        .expect("expect response");

    let withdraw_pool_amount = Uint128::new(120_000000u128);
    let pool_fee = Uint128::zero();
    let protocol_fee = Decimal::from_str("0.02").unwrap() * withdraw_pool_amount;
    let receive_amount = withdraw_pool_amount - pool_fee - protocol_fee;

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "arb/execute_withdraw"),
            attr("from", "cosmos2contract"),
            attr("receiver", "user001"),
            attr("withdraw_amount", withdraw_pool_amount),
            attr("receive_amount", receive_amount),
            attr("protocol_fee", protocol_fee),
            attr("pool_fee", pool_fee),
            attr("immediate", false.to_string()),
        ]
    );

    // withdraw + fee (without burn)
    assert_eq!(res.messages.len(), 2);

    match res.messages[0].msg.clone() {
        CosmosMsg::Bank(BankMsg::Send {
            to_address,
            amount,
        }) => {
            assert_eq!(to_address, "user001".to_string());
            assert_eq!(amount.len(), 1);
            assert_eq!(
                amount[0],
                Coin {
                    denom: "utoken".to_string(),
                    amount: receive_amount
                }
            );
        },

        _ => panic!("DO NOT ENTER HERE"),
    }

    match res.messages[1].msg.clone() {
        CosmosMsg::Bank(BankMsg::Send {
            to_address,
            amount,
        }) => {
            assert_eq!(to_address, "fee".to_string());
            assert_eq!(amount.len(), 1);
            assert_eq!(
                amount[0],
                Coin {
                    denom: "utoken".to_string(),
                    amount: protocol_fee
                }
            );
        },

        _ => panic!("DO NOT ENTER HERE"),
    }

    deps.querier.set_bank_balance(220_000000 - receive_amount.u128() - protocol_fee.u128());

    // share value is not changed, as there is no pool fee
    let share2 = query_utoken(deps.as_ref());
    assert_eq!(share, share2);

    let unbonding =
        query_unbond_requests(deps.as_ref(), end_time, "user001".to_string(), None, None).unwrap();

    // 1 item
    assert_eq!(
        unbonding,
        UnbondRequestsResponse {
            requests: vec![UnbondItem {
                start_time: 51,
                release_time: 51 + 100,
                amount_asset: Uint128::new(10_000000u128),
                id: 1,
                withdraw_protocol_fee: Uint128::new(200000),
                withdraw_pool_fee: Uint128::new(0u128),
                released: true
            }],
        }
    );

    //
    // WITHDRAW UNBONDED FAILED
    //
    let withdraw_unbonded = ExecuteMsg::WithdrawUnbonded {};

    let res = execute(deps.as_mut(), before_end_time, user, withdraw_unbonded).unwrap_err();

    assert_eq!(res, ContractError::NoWithdrawableAsset {});
}

#[test]
fn query_check_balances() {
    let (mut deps, _res) = _unbonding_slow_120();

    deps.querier.with_unbonding(Uint128::new(24_000000u128));
    deps.querier.with_withdrawable(Uint128::new(10_000000u128));

    let pool_available = Uint128::new(220_000000u128);
    let locked = Uint128::new(120_000000u128);
    let pool_takeable = pool_available - locked;

    let unbonding_per_lsd = Uint128::new(24_000000u128);
    let withdrawable_per_lsd = Uint128::new(10_000000u128);
    let eris_exchange_rate = Decimal::from_str("1.1").unwrap();
    let stader_exchange_rate = Decimal::from_str("1.02").unwrap();
    let unbonding = get_unbonding_value(unbonding_per_lsd.u128());
    let withdrawable = get_withdraw_value(withdrawable_per_lsd.u128());

    let total_value = pool_available + unbonding + withdrawable - locked;

    let balance = query_state(deps.as_ref(), mock_env(), None).unwrap();
    assert_eq!(
        balance,
        StateResponse {
            total_lp_supply: Uint128::new(100000000u128),
            balances: Balances {
                tvl_utoken: total_value + locked,
                vault_total: total_value,
                vault_available: pool_available,
                vault_takeable: pool_takeable,
                locked_user_withdrawls: locked,
                lsd_unbonding: unbonding,
                lsd_withdrawable: withdrawable
            },
            exchange_rate: Decimal::from_str("2.4008").unwrap(),
            details: None
        }
    );

    let balance_detail = query_state(deps.as_ref(), mock_env(), Some(true)).unwrap();
    assert_eq!(
        balance_detail,
        StateResponse {
            total_lp_supply: Uint128::new(100000000u128),
            balances: Balances {
                tvl_utoken: total_value + locked,
                vault_total: total_value,
                vault_available: pool_available,
                vault_takeable: pool_takeable,
                locked_user_withdrawls: locked,
                lsd_unbonding: unbonding,
                lsd_withdrawable: withdrawable
            },
            exchange_rate: Decimal::from_str("2.4008").unwrap(),
            details: Some(StateDetails {
                claims: vec![
                    ClaimBalance {
                        name: "eris".to_string(),
                        withdrawable: eris_exchange_rate * withdrawable_per_lsd,
                        unbonding: eris_exchange_rate * unbonding_per_lsd
                    },
                    ClaimBalance {
                        name: "steak".to_string(),
                        withdrawable: withdrawable_per_lsd,
                        unbonding: unbonding_per_lsd
                    },
                    ClaimBalance {
                        name: "stader".to_string(),
                        withdrawable: stader_exchange_rate * withdrawable_per_lsd,
                        unbonding: stader_exchange_rate * unbonding_per_lsd
                    },
                    ClaimBalance {
                        name: "prism".to_string(),
                        withdrawable: withdrawable_per_lsd,
                        unbonding: unbonding_per_lsd
                    },
                ],
                takeable_steps: vec![
                    // 1% = 50% of pool
                    (Decimal::from_ratio(10u128, 1000u128), Uint128::new(0u128),),
                    (Decimal::from_ratio(15u128, 1000u128), Uint128::new(27976000),),
                    (Decimal::from_ratio(20u128, 1000u128), Uint128::new(75992000),),
                    (Decimal::from_ratio(25u128, 1000u128), Uint128::new(100000000),),
                ]
            })
        }
    );
}

#[test]
fn query_check_available() {
    let (mut deps, _res) = _unbonding_slow_120();
    deps.querier.with_unbonding(Uint128::new(24_000000u128));
    deps.querier.with_withdrawable(Uint128::new(10_000000u128));

    let pool_available = Uint128::new(220_000000u128);
    let locked = Uint128::new(120_000000u128);
    let pool_takeable = pool_available - locked;
    let unbonding = get_unbonding_value(24_000000u128);
    let withdrawable = get_withdraw_value(10_000000u128);

    let total_value = pool_available + unbonding + withdrawable - locked;

    let available = query_takeable(deps.as_ref(), mock_env(), None).unwrap();

    assert_eq!(
        available,
        TakeableResponse {
            takeable: None,
            steps: vec![
                // 50%
                (
                    Decimal::from_ratio(10u128, 1000u128),
                    calc_takeable(total_value, pool_takeable, "0.5")
                ),
                // 70%
                (
                    Decimal::from_ratio(15u128, 1000u128),
                    calc_takeable(total_value, pool_takeable, "0.7")
                ),
                // 90%
                (
                    Decimal::from_ratio(20u128, 1000u128),
                    calc_takeable(total_value, pool_takeable, "0.9")
                ),
                (
                    Decimal::from_ratio(25u128, 1000u128),
                    calc_takeable(total_value, pool_takeable, "1.0")
                ),
            ],
        },
    );

    let available =
        query_takeable(deps.as_ref(), mock_env(), Some(Decimal::from_str("0.01").unwrap()))
            .unwrap();

    assert_eq!(
        available,
        TakeableResponse {
            takeable: Some(calc_takeable(total_value, pool_takeable, "0.5")),
            steps: vec![
                // 50%
                (
                    Decimal::from_ratio(10u128, 1000u128),
                    calc_takeable(total_value, pool_takeable, "0.5")
                ),
                // 70%
                (
                    Decimal::from_ratio(15u128, 1000u128),
                    calc_takeable(total_value, pool_takeable, "0.7")
                ),
                // 90%
                (
                    Decimal::from_ratio(20u128, 1000u128),
                    calc_takeable(total_value, pool_takeable, "0.9")
                ),
                (
                    Decimal::from_ratio(25u128, 1000u128),
                    calc_takeable(total_value, pool_takeable, "1.0")
                ),
            ],
        },
    );

    let available =
        query_takeable(deps.as_ref(), mock_env(), Some(Decimal::from_str("0.6").unwrap()))
            .unwrap_err();

    // currently no interpolation possible
    assert_eq!(available, ContractError::NotSupportedProfitStep(Decimal::from_str("0.6").unwrap()));
}

#[test]
fn execute_arb_throws() {
    let (mut deps, _res) = _unbonding_slow_120();

    deps.querier.with_unbonding(Uint128::new(24_000000u128));
    deps.querier.with_withdrawable(Uint128::new(10_000000u128));

    deps.querier.set_cw20_balance("lptoken", "share_user", 50_000000);
    let start_share = query_utoken(deps.as_ref());
    assert_eq!(start_share, Uint128::new(120_040000u128));

    let whitelist_info = mock_info("whitelisted_exec", &[]);
    let contract_info = mock_info(MOCK_CONTRACT_ADDR, &[]);

    let exec_msg = ExecuteMsg::ExecuteArbitrage {
        msg: ExecuteSubMsg {
            contract_addr: None,
            funds_amount: Uint128::new(1000_000000u128),
            msg: to_binary("exec_any_swap").unwrap(),
        },
        result_token: token_asset_info(Addr::unchecked("eriscw")),
        wanted_profit: Decimal::from_str("0.025").unwrap(),
    };
    let res = execute(deps.as_mut(), mock_env(), whitelist_info.clone(), exec_msg)
        .expect_err("expects error");
    assert_eq!(res, ContractError::NotEnoughFundsTakeable {});

    let exec_msg = ExecuteMsg::ExecuteArbitrage {
        msg: ExecuteSubMsg {
            contract_addr: None,
            funds_amount: Uint128::new(10_000000u128),
            msg: to_binary("exec_any_swap").unwrap(),
        },
        result_token: token_asset_info(Addr::unchecked("xxx")),
        wanted_profit: Decimal::from_str("0.025").unwrap(),
    };
    let res = execute(deps.as_mut(), mock_env(), whitelist_info.clone(), exec_msg)
        .expect_err("expects error");
    assert_eq!(res, ContractError::AssetUnknown {});

    let exec_msg = ExecuteMsg::ExecuteArbitrage {
        msg: ExecuteSubMsg {
            contract_addr: None,
            funds_amount: Uint128::zero(),
            msg: to_binary("exec_any_swap").unwrap(),
        },
        result_token: token_asset_info(Addr::unchecked("eriscw")),
        wanted_profit: Decimal::from_str("0.025").unwrap(),
    };
    let res = execute(deps.as_mut(), mock_env(), whitelist_info.clone(), exec_msg)
        .expect_err("expects error");
    assert_eq!(res, ContractError::InvalidZeroAmount {});

    let res = execute(
        deps.as_mut(),
        mock_env(),
        contract_info,
        ExecuteMsg::Callback(eris::arb_vault::CallbackMsg::AssertResult {
            result_token: token_asset_info(Addr::unchecked("eriscw")),
            wanted_profit: Decimal::from_str("0.01").unwrap(),
        }),
    )
    .unwrap_err();
    assert_eq!(res, ContractError::NotExecuting {});

    let wanted_profit = Decimal::from_str("0.015").unwrap();
    let takeable = query_takeable(deps.as_ref(), mock_env(), Some(wanted_profit))
        .unwrap()
        .takeable
        .expect("expects takeable");

    let exec_msg = ExecuteMsg::ExecuteArbitrage {
        msg: ExecuteSubMsg {
            contract_addr: None,
            funds_amount: takeable,
            msg: to_binary("exec_any_swap").unwrap(),
        },
        result_token: token_asset_info(Addr::unchecked("eriscw")),
        wanted_profit,
    };
    let res = execute(deps.as_mut(), mock_env(), whitelist_info, exec_msg).unwrap_err();
    assert_eq!(res, ContractError::WithdrawBeforeExecute {});
}

#[test]
fn execute_arb() {
    let (mut deps, _res) = _unbonding_slow_120();

    deps.querier.set_bank_balance(100_000000 + 120_000000);
    deps.querier.with_unbonding(Uint128::new(24_000000u128));
    deps.querier.with_withdrawable(Uint128::zero());

    let pool_available = Uint128::new(220_000000u128);
    let locked = Uint128::new(120_000000u128);
    let _pool_takeable = pool_available - locked;
    let unbonding = get_unbonding_value(24_000000u128);

    let old_tvl = pool_available + unbonding;

    let old_state = query_state(deps.as_ref(), mock_env(), None).unwrap();

    deps.querier.set_cw20_balance("lptoken", "share_user", 50_000000);
    let start_share = query_utoken(deps.as_ref());
    assert_eq!(start_share, Uint128::new(99_440000u128));

    let whitelist_info = mock_info("whitelisted_exec", &[]);
    let user_info = mock_info("user", &[]);
    let contract_info = mock_info(MOCK_CONTRACT_ADDR, &[]);

    let wanted_profit = Decimal::from_str("0.015").unwrap();
    let takeable = query_takeable(deps.as_ref(), mock_env(), Some(wanted_profit))
        .unwrap()
        .takeable
        .expect("expects takeable");

    let exec_msg = ExecuteMsg::ExecuteArbitrage {
        msg: ExecuteSubMsg {
            contract_addr: None,
            funds_amount: takeable,
            msg: to_binary("exec_any_swap").unwrap(),
        },
        result_token: token_asset_info(Addr::unchecked("eriscw")),
        wanted_profit,
    };
    let res = execute(deps.as_mut(), mock_env(), whitelist_info.clone(), exec_msg).unwrap();

    assert_eq!(res.attributes, vec![attr("action", "arb/execute_arbitrage")]);
    assert_eq!(res.messages.len(), 2);
    match res.messages[0].msg.clone() {
        CosmosMsg::Wasm(WasmMsg::Execute {
            funds,
            contract_addr,
            msg,
        }) => {
            assert_eq!(contract_addr, whitelist_info.sender.to_string());
            assert_eq!(
                funds,
                vec![Coin {
                    denom: "utoken".to_string(),
                    amount: takeable
                }]
            );

            let sub_msg: String = from_binary(&msg).unwrap();
            assert_eq!(sub_msg, "exec_any_swap");
        },
        _ => panic!("DO NOT ENTER HERE"),
    }

    let sub_msg: ExecuteMsg;
    match res.messages[1].msg.clone() {
        CosmosMsg::Wasm(WasmMsg::Execute {
            funds,
            contract_addr,
            msg,
        }) => {
            assert_eq!(contract_addr, MOCK_CONTRACT_ADDR.to_string());
            assert_eq!(funds.len(), 0);
            sub_msg = from_binary(&msg).unwrap();

            assert_eq!(
                sub_msg,
                ExecuteMsg::Callback(eris::arb_vault::CallbackMsg::AssertResult {
                    result_token: token_asset_info(Addr::unchecked("eriscw")),
                    wanted_profit
                })
            );
        },
        _ => panic!("DO NOT ENTER HERE"),
    }

    //
    // EXPECT PROVIDING LIQUIDITY WHILE EXECUTION TO THROW
    //

    let res = execute(
        deps.as_mut(),
        mock_env(),
        user_info,
        ExecuteMsg::ProvideLiquidity {
            asset: native_asset("utoken".to_string(), Uint128::new(100)),
            receiver: None,
        },
    )
    .unwrap_err();

    assert_eq!(res, ContractError::AlreadyExecuting {});

    let res = execute(
        deps.as_mut(),
        mock_env(),
        whitelist_info,
        ExecuteMsg::ExecuteArbitrage {
            msg: ExecuteSubMsg {
                contract_addr: None,
                msg: to_binary(&Empty {}).unwrap(),
                funds_amount: Uint128::new(100u128),
            },
            result_token: token_asset_info(Addr::unchecked("stadercw")),
            wanted_profit,
        },
    )
    .unwrap_err();

    assert_eq!(res, ContractError::AlreadyExecuting {});

    //
    // APPLYING SUB MSG TO NEW BALANCE
    //
    let profit_factor = Decimal::one() + wanted_profit;
    let eris_exchange_rate = Decimal::from_str("1.1").unwrap();
    // 100 bluna -> 101 luna
    let eris_amount = takeable * (profit_factor / eris_exchange_rate);

    // we have taken the takeable amount from the balance
    deps.querier.set_bank_balance(100_000000 + 120_000000 - takeable.u128());

    // and received the result in bluna

    deps.querier.set_cw20_balance("eriscw", MOCK_CONTRACT_ADDR, eris_amount.u128());

    //
    // END APPLYING SUB MSG TO NEW BALANCE
    //

    let res = execute(deps.as_mut(), mock_env(), contract_info, sub_msg).unwrap();

    let new_tvl = old_tvl + eris_exchange_rate * eris_amount - takeable;
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "arb/assert_result"),
            attr("type", "eris"),
            attr("old_tvl", old_tvl.to_string()),
            attr("new_tvl", new_tvl.to_string()),
            attr("used_balance", takeable.to_string()),
            attr("xbalance", eris_amount.to_string()),
            attr("unbond_xamount", eris_amount),
            attr("xfactor", "1.1"),
            attr("xvalue", eris_exchange_rate * eris_amount),
            //approx. profit
            attr("profit", "605039"),
            attr("exchange_rate", "1.99478989"),
            attr("fee_amount", "6050"),
        ]
    );

    //
    // APPLYING SUB MSG TO NEW BALANCE
    //

    // xasset moved to unbonding
    deps.querier.set_cw20_balance("eriscw", MOCK_CONTRACT_ADDR, 0);
    deps.querier.with_unbonding_eris(eris_amount);

    //
    // END APPLYING SUB MSG TO NEW BALANCE
    //

    assert_eq!(
        old_state,
        StateResponse {
            exchange_rate: Decimal::from_str("1.9888").unwrap(),
            total_lp_supply: Uint128::new(100000000),
            balances: Balances {
                tvl_utoken: old_tvl,
                vault_total: Uint128::new(198880000),
                vault_available: Uint128::new(220000000),
                vault_takeable: Uint128::new(100000000),
                locked_user_withdrawls: Uint128::new(120000000),
                lsd_unbonding: Uint128::new(98880000),
                lsd_withdrawable: Uint128::new(0),
            },
            details: None
        }
    );

    let new_state = query_state(deps.as_ref(), mock_env(), None).unwrap();
    assert_eq!(
        new_state,
        StateResponse {
            exchange_rate: Decimal::from_str("1.99485039").unwrap(),
            total_lp_supply: Uint128::new(100000000),
            balances: Balances {
                tvl_utoken: new_tvl,
                vault_total: Uint128::new(199485039),
                vault_available: Uint128::new(179664000),
                vault_takeable: Uint128::new(59664000),
                locked_user_withdrawls: Uint128::new(120000000),
                lsd_unbonding: Uint128::new(98880000 + (eris_amount * eris_exchange_rate).u128()),
                lsd_withdrawable: Uint128::new(0),
            },
            details: None
        }
    );

    assert_eq!(res.messages.len(), 2);

    match res.messages[0].msg.clone() {
        CosmosMsg::Wasm(WasmMsg::Execute {
            funds,
            contract_addr,
            msg,
        }) => {
            assert_eq!(contract_addr, "eriscw".to_string());
            assert_eq!(funds.len(), 0);
            let sub_msg: Cw20ExecuteMsg = from_binary(&msg).unwrap();

            if let Cw20ExecuteMsg::Send {
                amount,
                contract,
                msg,
            } = sub_msg
            {
                let sub_sub_msg: eris::hub::ReceiveMsg = from_binary(&msg).unwrap();
                assert_eq!(contract, "eris");
                assert_eq!(amount, eris_amount);
                assert_eq!(
                    sub_sub_msg,
                    eris::hub::ReceiveMsg::QueueUnbond {
                        receiver: None
                    }
                );
            } else {
                panic!("DO NOT ENTER HERE");
            }
        },
        _ => panic!("DO NOT ENTER HERE"),
    }

    assert_eq!(
        res.messages[1].msg,
        native_asset("utoken".to_string(), Uint128::new(6050))
            .into_msg(&deps.as_ref().querier, "fee")
            .unwrap()
    );

    //
    // EXPECT NEW SHARE TO BE BIGGER
    //
    let new_share = query_utoken(deps.as_ref());

    assert!(new_share.gt(&start_share), "new share must be bigger than start");
    assert_eq!(new_share, Uint128::new(99_742519u128));

    // expect takeable to be 0 afterwards
    let takeable =
        query_takeable(deps.as_ref(), mock_env(), Some(wanted_profit)).unwrap().takeable.unwrap();

    assert_eq!(takeable, Uint128::zero());
}

fn calc_takeable(total_value: Uint128, pool_takeable: Uint128, share: &str) -> Uint128 {
    // total value * share = total pool that can be used for that share
    // + takeable - total value

    // Example:
    // share = 0.7
    // total_value: 1000
    // total_value_for_profit 700
    // pool_takeable: 400
    // pool_takeable_for_profit -> 100 (total_for_profit+pool_takeable-total)
    (total_value * Decimal::from_str(share).expect("expect value"))
        .checked_add(pool_takeable)
        .unwrap_or(Uint128::zero())
        .checked_sub(total_value)
        .unwrap_or(Uint128::zero())
}

fn query_utoken(deps: Deps) -> Uint128 {
    let response = query_user_info(deps, mock_env(), "share_user".to_string()).unwrap();
    response.utoken_amount
}
