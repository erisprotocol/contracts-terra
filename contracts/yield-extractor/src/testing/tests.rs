use std::borrow::BorrowMut;
use std::ops::{Sub, Mul, Div, Add};
use std::str::FromStr;

use crate::contract::{execute, instantiate, reply, query};
use crate::math::{compute_mint_amount, compute_withdraw_amount};
use crate::state::State;
use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, to_binary, Addr, CosmosMsg, Decimal, Event, OwnedDeps, Reply, ReplyOn, StdError,
    SubMsg, SubMsgResponse, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, MinterResponse};
use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;
use cw_storage_plus::Item;
use eris_staking::yieldextractor::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, LiquidStakingType, QueryMsg, ReceiveMsg,
    StateResponse, ShareResponse,
};

use super::custom_querier::CustomQuerier;
use super::helpers::{mock_dependencies, mock_env_at_timestamp, query_helper};

//--------------------------------------------------------------------------------------------------
// Test setup
//--------------------------------------------------------------------------------------------------

fn setup_test() -> OwnedDeps<MockStorage, MockApi, CustomQuerier> {
    let mut deps = mock_dependencies();

    let res = instantiate(
        deps.as_mut(),
        mock_env_at_timestamp(10000),
        mock_info("deployer", &[]),
        InstantiateMsg {
            cw20_code_id: 69420,
            owner: "owner".to_string(),
            name: "ErisLP".to_string(),
            symbol: "LP".to_string(),
            decimals: 6,
            hub_contract: "hub".to_string(),
            stake_token: "stake".to_string(),
            interface: LiquidStakingType::Eris,
            yield_extract_addr: "yield".to_string(),
            yield_extract_p: Decimal::from_str("0.1").unwrap(),
            label: "Eris Yield Extraction LP Token".to_string(),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some("owner".to_string()),
                code_id: 69420,
                msg: to_binary(&Cw20InstantiateMsg {
                    name: "ErisLP".to_string(),
                    symbol: "LP".to_string(),
                    decimals: 6,
                    initial_balances: vec![],
                    mint: Some(MinterResponse {
                        minter: MOCK_CONTRACT_ADDR.to_string(),
                        cap: None
                    }),
                    marketing: None,
                })
                .unwrap(),
                funds: vec![],
                label: "Eris Yield Extraction LP Token".to_string(),
            }),
            1
        )
    );

    let event = Event::new("instantiate")
        .add_attribute("creator", MOCK_CONTRACT_ADDR)
        .add_attribute("admin", "admin")
        .add_attribute("code_id", "69420")
        .add_attribute("_contract_address", "lp_token");

    let res = reply(
        deps.as_mut(),
        mock_env_at_timestamp(10000),
        Reply {
            id: 1,
            result: cosmwasm_std::SubMsgResult::Ok(SubMsgResponse {
                events: vec![event],
                data: None,
            }),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 0);

    deps.querier.eris_querier.exchange_rate = Decimal::one();
    deps.querier.set_cw20_total_supply("lp_token", 0);
    deps.querier.set_cw20_total_supply("stake", 0);
    deps.querier.set_cw20_balance("stake", "cosmos2contract", 0);
    deps
}

//--------------------------------------------------------------------------------------------------
// Execution
//--------------------------------------------------------------------------------------------------

#[test]
fn proper_instantiation() {
    let deps = setup_test();

    let res: ConfigResponse = query_helper(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(
        res,
        ConfigResponse {
            owner: "owner".to_string(),
            new_owner: None,
            stake_token: "stake".to_string(),
            hub_contract: "hub".to_string(),
            interface: LiquidStakingType::Eris,
            lp_token: "lp_token".to_string(),
            yield_extract_addr: "yield".to_string(),
            yield_extract_p: Decimal::from_str("0.1").unwrap()
        }
    );

    let res: StateResponse = query_helper(deps.as_ref(), QueryMsg::State { addr: None});
    assert_eq!(
        res,
        StateResponse {
            tvl_uluna: Uint128::zero(),
            total_lp: Uint128::zero(),
            stake_balance: Uint128::zero(),
            stake_extracted: Uint128::zero(),
            stake_harvested: Uint128::zero(),
            exchange_rate_lp_stake: Decimal::from_str("0").unwrap(),
            exchange_rate_stake_uluna: Decimal::from_str("1").unwrap(),
            stake_available: Uint128::zero(),
            user_received_asset: None,
            user_share: None
        },
    );
}

#[test]
fn deposit() {
    let mut deps = setup_test();

    // Only Stake token is accepted for deposit requests
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("random_token", &[]),
        ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
            sender: "hacker".to_string(),
            amount: Uint128::new(69420),
            msg: to_binary(&ReceiveMsg::Deposit {}).unwrap(),
        }),
    )
    .unwrap_err();

    assert_eq!(err, StdError::generic_err("expecting Stake token, received random_token"));

    deps.querier.set_cw20_balance("stake", "cosmos2contract", 100);
    // Only Stake token is accepted for deposit requests
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("stake", &[]),
        ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
            sender: "user_1".to_string(),
            amount: Uint128::new(100),
            msg: to_binary(&ReceiveMsg::Deposit {}).unwrap(),
        }),
    )
    .unwrap();

    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "lp_token".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: "user_1".to_string(),
                    amount: Uint128::new(100)
                })
                .unwrap(),
                funds: vec![]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never,
        }
    );
}

#[test]
fn deposit_withdraw_same() {
    let mut deps = setup_test();

    deps.querier.set_cw20_balance("stake", "cosmos2contract", 100);

    // Only Stake token is accepted for deposit requests
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("stake", &[]),
        ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
            sender: "user_1".to_string(),
            amount: Uint128::new(100),
            msg: to_binary(&ReceiveMsg::Deposit {}).unwrap(),
        }),
    )
    .unwrap();

    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "lp_token".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: "user_1".to_string(),
                    amount: Uint128::new(100)
                })
                .unwrap(),
                funds: vec![]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never,
        }
    );

    // after initial deposit, 100 lp_token and 100 stake balance (+50 for next deposit)
    deps.querier.set_cw20_total_supply("lp_token", 100);
    deps.querier.set_cw20_balance("stake", "cosmos2contract", 100 + 50);
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("stake", &[]),
        ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
            sender: "user_2".to_string(),
            amount: Uint128::new(50),
            msg: to_binary(&ReceiveMsg::Deposit {}).unwrap(),
        }),
    )
    .unwrap();

    assert_eq!(res.messages.len(), 1);

    match res.messages[0].msg.clone() {
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            funds,
            msg,
        }) => {
            assert_eq!(contract_addr, "lp_token".to_string());
            assert_eq!(funds.len(), 0);

            let sub_msg: Cw20ExecuteMsg = from_binary(&msg).unwrap();

            assert_eq!(
                sub_msg,
                Cw20ExecuteMsg::Mint {
                    recipient: "user_2".to_string(),
                    amount: Uint128::new(50)
                }
            );
        },

        _ => panic!("DO NOT ENTER HERE"),
    }

    // Only lptoken token is accepted for withdraw requests
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("random_token", &[]),
        ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
            sender: "hacker".to_string(),
            amount: Uint128::new(69420),
            msg: to_binary(&ReceiveMsg::Withdraw {}).unwrap(),
        }),
    )
    .unwrap_err();

    assert_eq!(err, StdError::generic_err("expecting LP token, received random_token"));

    deps.querier.set_cw20_total_supply("lp_token", 100 + 50);
    deps.querier.set_cw20_balance("lp_token", "cosmos2contract", 50);
    deps.querier.set_cw20_balance("stake", "cosmos2contract", 100 + 50);
    // Only lptoken token is accepted for deposit requests
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("lp_token", &[]),
        ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
            sender: "user_1".to_string(),
            amount: Uint128::new(50),
            msg: to_binary(&ReceiveMsg::Withdraw {}).unwrap(),
        }),
    )
    .unwrap();

    assert_eq!(res.messages.len(), 2);
    assert_eq!(
        res.messages[0],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "lp_token".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: Uint128::new(50)
                })
                .unwrap(),
                funds: vec![]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never,
        }
    );

    match res.messages[1].msg.clone() {
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            funds,
            msg,
        }) => {
            assert_eq!(contract_addr, "stake".to_string());
            assert_eq!(funds.len(), 0);

            let sub_msg: Cw20ExecuteMsg = from_binary(&msg).unwrap();

            assert_eq!(
                sub_msg,
                Cw20ExecuteMsg::Transfer {
                    recipient: "user_1".to_string(),
                    amount: Uint128::new(50)
                }
            );
        },

        _ => panic!("DO NOT ENTER HERE"),
    }
}


#[test]
fn deposit_extract_withdraw() {
    let mut deps = setup_test();

    deps.querier.set_cw20_balance("stake", "cosmos2contract", 100_000000);

    // Only Stake token is accepted for deposit requests
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("stake", &[]),
        ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
            sender: "user_1".to_string(),
            amount: Uint128::new(100_000000),
            msg: to_binary(&ReceiveMsg::Deposit {}).unwrap(),
        }),
    )
    .unwrap();

    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "lp_token".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: "user_1".to_string(),
                    amount: Uint128::new(100_000000)
                })
                .unwrap(),
                funds: vec![]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never,
        }
    );

    // after initial deposit, 100 lp_token and 100 stake balance (+50 for next deposit)
    deps.querier.set_cw20_total_supply("lp_token", 100_000000);
    deps.querier.set_cw20_balance("lp_token", "cosmos2contract", 0_000000);
    deps.querier.set_cw20_balance("stake", "cosmos2contract", 100_000000 + 50_000000);
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("stake", &[]),
        ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
            sender: "user_2".to_string(),
            amount: Uint128::new(50_000000),
            msg: to_binary(&ReceiveMsg::Deposit {}).unwrap(),
        }),
    )
    .unwrap();

    assert_eq!(res.messages.len(), 1);

    match res.messages[0].msg.clone() {
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            funds,
            msg,
        }) => {
            assert_eq!(contract_addr, "lp_token".to_string());
            assert_eq!(funds.len(), 0);

            let sub_msg: Cw20ExecuteMsg = from_binary(&msg).unwrap();

            assert_eq!(
                sub_msg,
                Cw20ExecuteMsg::Mint {
                    recipient: "user_2".to_string(),
                    amount: Uint128::new(50_000000)
                }
            );
        },

        _ => panic!("DO NOT ENTER HERE"),
    }

    deps.querier.set_cw20_total_supply("lp_token", 100_000000 + 50_000000);
    deps.querier.set_cw20_balance("lp_token", "user_1", 100_000000);
    deps.querier.set_cw20_balance("lp_token", "user_2", 50_000000);
    
    let share: ShareResponse = query_helper(deps.as_ref(), QueryMsg::Share { addr: Some("user_1".to_string()) });
    assert_eq!(share, ShareResponse { 
        received_asset: Uint128::new(100_000000), 
        share: Uint128::new(100_000000), 
        total_lp: Uint128::new(150_000000) });


    // value increased by 5 %, 10% extraction parameter
    deps.querier.eris_querier.exchange_rate = Decimal::from_str("1.05").unwrap();

    deps.querier.set_cw20_balance("lp_token", "user_1", 50_000000);
    deps.querier.set_cw20_balance("lp_token", "cosmos2contract", 50_000000);
    deps.querier.set_cw20_balance("stake", "cosmos2contract", 100_000000 + 50_000000);
    // Only lptoken token is accepted for deposit requests
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("lp_token", &[]),
        ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
            sender: "user_1".to_string(),
            amount: Uint128::new(50_000000),
            msg: to_binary(&ReceiveMsg::Withdraw {}).unwrap(),
        }),
    )
    .unwrap();

    assert_eq!(res.messages.len(), 2);
    assert_eq!(
        res.messages[0],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "lp_token".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: Uint128::new(50_000000)
                })
                .unwrap(),
                funds: vec![]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never,
        }
    );

    match res.messages[1].msg.clone() {
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            funds,
            msg,
        }) => {
            assert_eq!(contract_addr, "stake".to_string());
            assert_eq!(funds.len(), 0);

            let sub_msg: Cw20ExecuteMsg = from_binary(&msg).unwrap();

            // 49.75
            let extracted = 
                // 5% yield
                Decimal::from_str("0.05").unwrap()
                // 10% extraction
                * Decimal::from_str("0.1").unwrap()
                // 50 withdraw
                * Uint128::new(50_000000);
            
            let received = Uint128::new(50_000000).sub(extracted);

            assert_eq!(
                sub_msg,
                Cw20ExecuteMsg::Transfer {
                    recipient: "user_1".to_string(),
                    amount: received
                }
            );
        },

        _ => panic!("DO NOT ENTER HERE"),
    }
    deps.querier.set_cw20_total_supply("lp_token", 100_000000 + 50_000000 - 50_000000);
    deps.querier.set_cw20_balance("lp_token", "cosmos2contract", 0_000000);
    deps.querier.set_cw20_balance("stake", "cosmos2contract", 100_000000 + 50_000000 - 49_750000);

    let share: ShareResponse = query_helper(deps.as_ref(), QueryMsg::Share { addr: Some("user_1".to_string()) });

    assert_eq!(share, ShareResponse { 
        received_asset: Uint128::new(49_750000), 
        share: Uint128::new(50_000000), 
        total_lp: Uint128::new(100_000000) });

    let state: StateResponse = query_helper(deps.as_ref(), QueryMsg::State { addr: None });

    assert_eq!(state, 
        StateResponse {
            tvl_uluna: Uint128::new(105262500),
            total_lp: Uint128::new(100000000),
            stake_balance: Uint128::new(100250000),
            stake_extracted: Uint128::new(750000),
            stake_harvested: Uint128::zero(),
            exchange_rate_lp_stake: Decimal::from_str("0.995").unwrap(),
            exchange_rate_stake_uluna: Decimal::from_str("1.05").unwrap(),
            
            stake_available: Uint128::new(99_500000),
            user_received_asset: None,
            user_share: None
        }
    );


    // harvest
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("anyone", &[]),
        ExecuteMsg::Harvest {  },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "stake".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    amount: Uint128::new(750000),
                    recipient: "yield".to_string()
                })
                .unwrap(),
                funds: vec![]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never,
        }
    );
    
    deps.querier.set_cw20_balance("stake", "cosmos2contract", 100_000000 + 50_000000 - 49_750000 - 750000);

    // no change in the share after harvest, only if exchange rate would have changed.
    let share: ShareResponse = query_helper(deps.as_ref(), QueryMsg::Share { addr: Some("user_1".to_string()) });

    assert_eq!(share, ShareResponse { 
        received_asset: Uint128::new(49_750000), 
        share: Uint128::new(50_000000), 
        total_lp: Uint128::new(100_000000) });

        
    // no change in the share after harvest, only if exchange rate would have changed.
    let share: ShareResponse = query_helper(deps.as_ref(), QueryMsg::Share { addr: None });

    assert_eq!(share, ShareResponse { 
        received_asset: Uint128::new(0), 
        share: Uint128::new(0), 
        total_lp: Uint128::new(100_000000) });


        let state: StateResponse = query_helper(deps.as_ref(), QueryMsg::State { addr: None });

        assert_eq!(state, 
            StateResponse {
                tvl_uluna: Uint128::new(104475000),
                total_lp: Uint128::new(100000000),
                stake_balance: Uint128::new(99500000),
                stake_extracted: Uint128::zero(),
                stake_harvested: Uint128::new(750000),
                exchange_rate_lp_stake: Decimal::from_str("0.995").unwrap(),
                exchange_rate_stake_uluna: Decimal::from_str("1.05").unwrap(),
                stake_available: Uint128::new(99_500000),
                user_received_asset: None,
                user_share: None
            }
        );

        let state: StateResponse = query_helper(deps.as_ref(), QueryMsg::State { addr: Some("user_1".to_string()) });
    
        assert_eq!(state, 
            StateResponse {
                tvl_uluna: Uint128::new(104475000),
                total_lp: Uint128::new(100000000),
                stake_balance: Uint128::new(99500000),
                stake_extracted: Uint128::zero(),
                stake_harvested: Uint128::new(750000),
                exchange_rate_lp_stake: Decimal::from_str("0.995").unwrap(),
                exchange_rate_stake_uluna: Decimal::from_str("1.05").unwrap(),
                stake_available: Uint128::new(99_500000),
                user_received_asset: Some(Uint128::new(49_750000)),
                user_share: Some(Uint128::new(50_000000))
            }
        );

}


#[test]
fn test_query_state() {
    let mut deps = setup_test();

    deps.querier.eris_querier.exchange_rate = Decimal::from_ratio(10u128, 1u128);
    
    deps.querier.set_cw20_total_supply("lp_token", 0);
    deps.querier.set_cw20_balance("lp_token", "cosmos2contract", 0);
    deps.querier.set_cw20_balance("stake", "cosmos2contract", 100_000000);

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("stake", &[]),
        ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
            sender: "user_1".to_string(),
            amount: Uint128::new(100_000000),
            msg: to_binary(&ReceiveMsg::Deposit {}).unwrap(),
        }),
    )
    .unwrap();

    deps.querier.set_cw20_total_supply("lp_token", 100_000000);
    deps.querier.set_cw20_balance("lp_token", "user_1", 100_000000);

    let state = State::default();

    let exchange_rate = state.last_exchange_rate.load(deps.as_ref().storage).unwrap();
    assert_eq!(exchange_rate, Decimal::from_ratio(10u128, 1u128));
    
    let res: StateResponse = query_helper(deps.as_ref(), QueryMsg::State { addr: Some("user_1".to_string()) });

    // 1000 LUNA -> 1000 LUNA
    // 100 ampLUNA -> 100 ampLUNA
    // 0.1 Extract * 0 LUNA (diff) = 0 LUNA = 0 ampLUNA
    // user has 100 - 0 = 100 ampLUNA = 1000
    assert_eq!(res, StateResponse {
        exchange_rate_lp_stake: Decimal::one(),
        exchange_rate_stake_uluna: Decimal::from_ratio(10u128, 1u128),
        stake_available: Uint128::from(100_000000u128),
        stake_balance: Uint128::from(100_000000u128),
        stake_extracted: Uint128::zero(),
        stake_harvested: Uint128::zero(),
        total_lp: Uint128::from(100_000000u128),
        tvl_uluna: Uint128::from(1000_000000u128),
        user_received_asset: Some(Uint128::from(100_000000u128)),
        user_share: Some(Uint128::from(100_000000u128))
    });

    // exchange_rate has increased
    deps.querier.eris_querier.exchange_rate = Decimal::from_ratio(20u128, 1u128);
    let res: StateResponse = query_helper(deps.as_ref(), QueryMsg::State { addr: Some("user_1".to_string()) });

    // 1000 LUNA -> 2000 LUNA
    // 100 ampLUNA -> 100 ampLUNA
    // 0.1 Extract * 1000 LUNA (diff) = 100 LUNA = 5 ampLUNA
    // user has 100 - 5 = 95 ampLUNA = 1900
    assert_eq!(res, StateResponse {
        exchange_rate_lp_stake: Decimal::from_ratio(95u128, 100u128),
        exchange_rate_stake_uluna: Decimal::from_ratio(20u128, 1u128),
        stake_available: Uint128::from(95_000000u128),
        stake_balance: Uint128::from(100_000000u128),
        stake_extracted: Uint128::from(5_000000u128),
        stake_harvested: Uint128::zero(),
        total_lp: Uint128::from(100_000000u128),
        tvl_uluna: Uint128::from(2000_000000u128),
        user_received_asset: Some(Uint128::from(95_000000u128)),
        user_share: Some(Uint128::from(100_000000u128))
    });

    
    // exchange_rate has increased
    deps.querier.eris_querier.exchange_rate = Decimal::from_ratio(25u128, 1u128);
    let res: StateResponse = query_helper(deps.as_ref(), QueryMsg::State { addr: Some("user_1".to_string()) });

    // 2000 LUNA -> 2500 LUNA
    // 100 ampLUNA -> 100 ampLUNA
    // 0.1 Extract * 1500 LUNA (diff) = 150 LUNA = 6 ampLUNA
    // user has 100 - 6 = 94 ampLUNA = 2350
    assert_eq!(res, StateResponse {
        exchange_rate_lp_stake: Decimal::from_ratio(94u128, 100u128),
        exchange_rate_stake_uluna: Decimal::from_ratio(25u128, 1u128),
        stake_available: Uint128::from(94_000000u128),
        stake_balance: Uint128::from(100_000000u128),
        stake_extracted: Uint128::from(6_000000u128),
        stake_harvested: Uint128::zero(),
        total_lp: Uint128::from(100_000000u128),
        tvl_uluna: Uint128::from(2500_000000u128),
        user_received_asset: Some(Uint128::from(94_000000u128)),
        user_share: Some(Uint128::from(100_000000u128))
    });

}


#[test]
fn deposit_test_real() {
    
    
    let mut deps = setup_test();

    deps.querier.eris_querier.exchange_rate = Decimal::from_ratio(10u128, 1u128);
    
    deps.querier.set_cw20_total_supply("lp_token", 0);
    deps.querier.set_cw20_balance("lp_token", "cosmos2contract", 0);
    deps.querier.set_cw20_balance("stake", "cosmos2contract", 100_000000);

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("stake", &[]),
        ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
            sender: "user_1".to_string(),
            amount: Uint128::new(100_000000),
            msg: to_binary(&ReceiveMsg::Deposit {}).unwrap(),
        }),
    )
    .unwrap();

    deps.querier.set_cw20_total_supply("lp_token", 100_000000);
    deps.querier.set_cw20_balance("lp_token", "user_1", 100_000000);

    let state = State::default();

    let exchange_rate = state.last_exchange_rate.load(deps.as_ref().storage).unwrap();
    assert_eq!(exchange_rate, Decimal::from_ratio(10u128, 1u128));
    
    let res: StateResponse = query_helper(deps.as_ref(), QueryMsg::State { addr: Some("user_1".to_string()) });

    // 1000 LUNA -> 1000 LUNA
    // 100 ampLUNA -> 100 ampLUNA
    // 0.1 Extract * 0 LUNA (diff) = 0 LUNA = 0 ampLUNA
    // user has 100 - 0 = 100 ampLUNA = 1000
    assert_eq!(res, StateResponse {
        exchange_rate_lp_stake: Decimal::one(),
        exchange_rate_stake_uluna: Decimal::from_ratio(10u128, 1u128),
        stake_available: Uint128::from(100_000000u128),
        stake_balance: Uint128::from(100_000000u128),
        stake_extracted: Uint128::zero(),
        stake_harvested: Uint128::zero(),
        total_lp: Uint128::from(100_000000u128),
        tvl_uluna: Uint128::from(1000_000000u128),
        user_received_asset: Some(Uint128::from(100_000000u128)),
        user_share: Some(Uint128::from(100_000000u128))
    });

    // exchange_rate has increased
    deps.querier.eris_querier.exchange_rate = Decimal::from_ratio(20u128, 1u128);

    let res: StateResponse = query_helper(deps.as_ref(), QueryMsg::State { addr: Some("user_1".to_string()) });

    // 1000 LUNA -> 2000 LUNA
    // 100 ampLUNA -> 100 ampLUNA
    // 0.1 Extract * 1000 LUNA (diff) = 100 LUNA = 5 ampLUNA
    // user has 100 - 5 = 95 ampLUNA = 1900
    assert_eq!(res, StateResponse {
        exchange_rate_lp_stake: Decimal::from_ratio(95u128, 100u128),
        exchange_rate_stake_uluna: Decimal::from_ratio(20u128, 1u128),
        stake_available: Uint128::from(95_000000u128),
        stake_balance: Uint128::from(100_000000u128),
        stake_extracted: Uint128::from(5_000000u128),
        stake_harvested: Uint128::zero(),
        total_lp: Uint128::from(100_000000u128),
        tvl_uluna: Uint128::from(2000_000000u128),
        user_received_asset: Some(Uint128::from(95_000000u128)),
        user_share: Some(Uint128::from(100_000000u128))
    });

    // another user deposits 100 stake
    deps.querier.set_cw20_balance("stake", "cosmos2contract", 200_000000);
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("stake", &[]),
        ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
            sender: "user_2".to_string(),
            amount: Uint128::new(100_000000),
            msg: to_binary(&ReceiveMsg::Deposit {}).unwrap(),
        }),
    )
    .unwrap();

    let added = Uint128::new(100_000000) * Decimal::from_ratio(100u128, 95u128);
    let total_lp = Uint128::from(100_000000u128) + added;

    deps.querier.set_cw20_total_supply("lp_token", total_lp.u128());
    deps.querier.set_cw20_balance("lp_token", "user_2", added.u128());


    
    let res: StateResponse = query_helper(deps.as_ref(), QueryMsg::State { addr: Some("user_2".to_string()) });

    assert_eq!(res, StateResponse {
        exchange_rate_lp_stake: Decimal::from_ratio(950000004141025659u128, 1000000000000000000u128),
        exchange_rate_stake_uluna: Decimal::from_ratio(20u128, 1u128),
        stake_available: Uint128::from(195_000000u128),
        stake_balance: Uint128::from(200_000000u128),
        stake_extracted: Uint128::from(5_000000u128),
        stake_harvested: Uint128::zero(),
        total_lp,
        tvl_uluna: Uint128::from(4000_000000u128),
        user_received_asset: Some(Uint128::from(99999999u128)),
        user_share: Some(added)
    });


    // NO CHANGE TO USER 1 when the same exchange rate    
    let res: StateResponse = query_helper(deps.as_ref(), QueryMsg::State { addr: Some("user_1".to_string()) });

    // 1000 LUNA -> 2000 LUNA
    // 100 ampLUNA -> 100 ampLUNA
    // 0.1 Extract * 1000 LUNA (diff) = 100 LUNA = 5 ampLUNA
    // user has 100 - 5 = 95 ampLUNA = 1900
    assert_eq!(res, StateResponse {
        exchange_rate_lp_stake: Decimal::from_ratio(950000004141025659u128, 1000000000000000000u128),
        exchange_rate_stake_uluna: Decimal::from_ratio(20u128, 1u128),
        stake_available: Uint128::from(195_000000u128),
        stake_balance: Uint128::from(200_000000u128),
        stake_extracted: Uint128::from(5_000000u128),
        stake_harvested: Uint128::zero(),
        total_lp,
        tvl_uluna: Uint128::from(4000_000000u128),
        user_received_asset: Some(Uint128::from(95_000000u128)),
        user_share: Some(Uint128::from(100_000000u128))
    });


    
    // exchange_rate has increased
    deps.querier.eris_querier.exchange_rate = Decimal::from_ratio(25u128, 1u128);
    let res: StateResponse = query_helper(deps.as_ref(), QueryMsg::State { addr: Some("user_1".to_string()) });

    
    // when the exchange rate changes again, the user will have less, than with single step
    // it is because the extraction @20 already took some ampLUNA out from the user into the protocol
    // that means the user is "less" invested into luna and will not participate as much in the compounding 

    //old   new	extracted	total	available	val before	val	    got	    extr.	LUNA (extr.)	diff	ampLUNA extracted
    //10	20	0	        100	    100	        1000	    2000	1000	10%	    100	            0,5	    5
    //20	25	5	        100	    95	        1900	    2375	475	    10%	    47,5	        0,2	    1,9
    //10	25	0	        100	    100	        1000	    2500	1500	10%	    150	            0,6	    6
    //20	25	5	        200	    195	        3900	    4875	975	    10%	    97,5	        0,2	    3,9

    // 10-20 and then 20-25 will lead to an extraction of 6.9
    // while
    // 10-25 will only lead to an extraction of 6

    assert_eq!(res, StateResponse {
        exchange_rate_lp_stake: Decimal::from_ratio(931000004058205145u128, 1000000000000000000u128),
        exchange_rate_stake_uluna: Decimal::from_ratio(25u128, 1u128),
        stake_available: Uint128::from(191_100000u128),
        stake_balance: Uint128::from(200_000000u128),
        stake_extracted: Uint128::from(8_900000u128),
        stake_harvested: Uint128::zero(),
        total_lp,
        tvl_uluna: Uint128::from(5000_000000u128),
        user_received_asset: Some(Uint128::from(93_100000u128)),
        user_share: Some(Uint128::from(100_000000u128))
    });


}

//--------------------------------------------------------------------------------------------------
// Manage
//--------------------------------------------------------------------------------------------------

#[test]
fn transferring_ownership() {
    let mut deps = setup_test();
    let state = State::default();

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake", &[]),
        ExecuteMsg::TransferOwnership {
            new_owner: "jake".to_string(),
        },
    )
    .unwrap_err();

    assert_eq!(err, StdError::generic_err("unauthorized: sender is not owner"));

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::TransferOwnership {
            new_owner: "jake".to_string(),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 0);

    let owner = state.owner.load(deps.as_ref().storage).unwrap();
    assert_eq!(owner, Addr::unchecked("owner"));

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("pumpkin", &[]),
        ExecuteMsg::AcceptOwnership {},
    )
    .unwrap_err();

    assert_eq!(err, StdError::generic_err("unauthorized: sender is not new owner"));

    let res =
        execute(deps.as_mut(), mock_env(), mock_info("jake", &[]), ExecuteMsg::AcceptOwnership {})
            .unwrap();

    assert_eq!(res.messages.len(), 0);

    let owner = state.owner.load(deps.as_ref().storage).unwrap();
    assert_eq!(owner, Addr::unchecked("jake"));
}



#[test]
fn update_config() {
    let mut deps = setup_test();
    let state = State::default();

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake", &[]),
        ExecuteMsg::UpdateConfig {
            yield_extract_addr: None,
        },
    )
    .unwrap_err();

    assert_eq!(err, StdError::generic_err("unauthorized: sender is not owner"));

    
    
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::UpdateConfig {
            yield_extract_addr: Some("new".to_string()),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 0);
    assert_eq!(state.extract_config.load(deps.as_ref().storage).unwrap().yield_extract_addr, Addr::unchecked("new".to_string()));
}


//--------------------------------------------------------------------------------------------------
// Queries
//--------------------------------------------------------------------------------------------------

//--------------------------------------------------------------------------------------------------
// Libraries
//--------------------------------------------------------------------------------------------------

#[test]
fn test_compute_mint_amount() {
    let result = compute_mint_amount(
        Uint128::from(1000u128),
        Uint128::from(100u128),
        Uint128::from(1000u128),
    );
    assert_eq!(result, Uint128::from(100u128));

    let result = compute_mint_amount(
        Uint128::from(1100u128),
        Uint128::from(100u128),
        Uint128::from(1100u128),
    );
    assert_eq!(result, Uint128::from(100u128));

    let result = compute_mint_amount(
        Uint128::from(1000u128),
        Uint128::from(100u128),
        Uint128::from(1100u128),
    );
    assert_eq!(result, Uint128::from(90u128));
}

#[test]
fn test_compute_withdraw_amount() {
    let result = compute_withdraw_amount(
        Uint128::from(1100u128),
        Uint128::from(100u128),
        Uint128::from(1100u128),
    );
    assert_eq!(result, Uint128::from(100u128));

    let result = compute_withdraw_amount(
        Uint128::from(1000u128),
        Uint128::from(100u128),
        Uint128::from(1100u128),
    );
    assert_eq!(result, Uint128::from(110u128));
}
