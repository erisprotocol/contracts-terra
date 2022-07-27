use std::ops::Sub;
use std::str::FromStr;

use crate::contract::{execute, instantiate, reply};
use crate::math::{compute_mint_amount, compute_withdraw_amount};
use crate::state::State;
use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, to_binary, Addr, CosmosMsg, Decimal, Event, OwnedDeps, Reply, ReplyOn, StdError,
    SubMsg, SubMsgResponse, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, MinterResponse};
use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;
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

    let res: StateResponse = query_helper(deps.as_ref(), QueryMsg::State {});
    assert_eq!(
        res,
        StateResponse {
            tvl_uluna: Uint128::zero(),
            total_lp: Uint128::zero(),
            total_lsd: Uint128::zero(),
            harvestable: Uint128::zero(),
            total_harvest: Uint128::zero(),
            exchange_rate_lp_lsd: Decimal::from_str("0").unwrap(),
            exchange_rate_lsd_uluna: Decimal::from_str("1").unwrap(),
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
    
    let share: ShareResponse = query_helper(deps.as_ref(), QueryMsg::Share { addr: "user_1".to_string() });
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

    let share: ShareResponse = query_helper(deps.as_ref(), QueryMsg::Share { addr: "user_1".to_string() });

    assert_eq!(share, ShareResponse { 
        received_asset: Uint128::new(49_750000), 
        share: Uint128::new(50_000000), 
        total_lp: Uint128::new(100_000000) });

    let state: StateResponse = query_helper(deps.as_ref(), QueryMsg::State {  });

    assert_eq!(state, 
        StateResponse {
            tvl_uluna: Uint128::new(105262500),
            total_lp: Uint128::new(100000000),
            total_lsd: Uint128::new(100250000),
            harvestable: Uint128::new(750000),
            total_harvest: Uint128::zero(),
            exchange_rate_lp_lsd: Decimal::from_str("0.995").unwrap(),
            exchange_rate_lsd_uluna: Decimal::from_str("1.05").unwrap(),
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
    let share: ShareResponse = query_helper(deps.as_ref(), QueryMsg::Share { addr: "user_1".to_string() });

    assert_eq!(share, ShareResponse { 
        received_asset: Uint128::new(49_750000), 
        share: Uint128::new(50_000000), 
        total_lp: Uint128::new(100_000000) });

    let state: StateResponse = query_helper(deps.as_ref(), QueryMsg::State {  });

    assert_eq!(state, 
        StateResponse {
            tvl_uluna: Uint128::new(104475000),
            total_lp: Uint128::new(100000000),
            total_lsd: Uint128::new(99500000),
            harvestable: Uint128::zero(),
            total_harvest: Uint128::new(750000),
            exchange_rate_lp_lsd: Decimal::from_str("0.995").unwrap(),
            exchange_rate_lsd_uluna: Decimal::from_str("1.05").unwrap(),
        }
    );

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
            yield_extract_p: Some(Decimal::from_str("2.0").unwrap()),
        },
    )
    .unwrap_err();

    assert_eq!(err, StdError::generic_err("unauthorized: sender is not owner"));

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::UpdateConfig {
            yield_extract_addr: None,
            yield_extract_p: Some(Decimal::from_str("2.0").unwrap()),
        },
    )
    .unwrap_err();

    assert_eq!(err, StdError::generic_err("'yield_extract' greater than max"));

    
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::UpdateConfig {
            yield_extract_addr: None,
            yield_extract_p: Some(Decimal::from_str("1.0").unwrap()),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 0);
    assert_eq!(state.extract_config.load(deps.as_ref().storage).unwrap().yield_extract_p, Decimal::from_str("1.0").unwrap());
    
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::UpdateConfig {
            yield_extract_addr: Some("new".to_string()),
            yield_extract_p: None,
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
