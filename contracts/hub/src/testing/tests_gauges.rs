use std::str::FromStr;
use std::vec;

use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    coin, to_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, DistributionMsg, Event, Order,
    OwnedDeps, Reply, StdError, StdResult, SubMsg, SubMsgResponse, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, MinterResponse};
use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;
use eris::DecimalCheckedOps;

use eris::hub::{
    Batch, CallbackMsg, ConfigResponse, ExecuteMsg, FeeConfig, InstantiateMsg, PendingBatch,
    QueryMsg, ReceiveMsg, StateResponse, UnbondRequest, UnbondRequestsByBatchResponseItem,
    UnbondRequestsByUserResponseItem, UnbondRequestsByUserResponseItemDetails,
};
use itertools::Itertools;

use crate::constants::CONTRACT_DENOM;
use crate::contract::{execute, instantiate, reply};
use crate::error::ContractError;
use crate::helpers::{dedupe, parse_received_fund};
use crate::math::{
    compute_redelegations_for_rebalancing, compute_redelegations_for_removal, compute_undelegations,
};
use crate::state::State;
use crate::testing::helpers::query_helper_env;
use crate::types::{Coins, Delegation, Redelegation, SendFee, Undelegation};

use super::custom_querier::CustomQuerier;
use super::helpers::{mock_dependencies, mock_env_at_timestamp, query_helper};

pub const STAKE_DENOM: &str = "stake_token";

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
            name: "Stake Token".to_string(),
            symbol: "STAKE".to_string(),
            decimals: 6,
            epoch_period: 259200,   // 3 * 24 * 60 * 60 = 3 days
            unbond_period: 1814400, // 21 * 24 * 60 * 60 = 21 days
            validators: vec!["alice".to_string(), "bob".to_string(), "charlie".to_string()],
            protocol_fee_contract: "fee".to_string(),
            protocol_reward_fee: Decimal::from_ratio(1u128, 100u128),
            delegation_strategy: Some(eris::hub::DelegationStrategy::Gauges {
                amp_gauges: "amp_gauges".to_string(),
                emp_gauges: Some("emp_gauges".to_string()),
                amp_factor_bps: 5000,
                min_delegation_bps: 100,
                max_delegation_bps: 2500,
                validator_count: 5,
            }),
            vote_operator: None,
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
                    name: "Stake Token".to_string(),
                    symbol: "STAKE".to_string(),
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
                label: "Eris Liquid Staking Token".to_string(),
            }),
            1
        )
    );

    let event = Event::new("instantiate")
        .add_attribute("creator", MOCK_CONTRACT_ADDR)
        .add_attribute("admin", "admin")
        .add_attribute("code_id", "69420")
        .add_attribute("_contract_address", STAKE_DENOM);

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

    deps.querier.set_cw20_total_supply(STAKE_DENOM, 0);
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
            stake_token: STAKE_DENOM.to_string(),
            epoch_period: 259200,
            unbond_period: 1814400,
            validators: vec!["alice".to_string(), "bob".to_string(), "charlie".to_string()],
            fee_config: FeeConfig {
                protocol_fee_contract: Addr::unchecked("fee"),
                protocol_reward_fee: Decimal::from_ratio(1u128, 100u128)
            },
            delegation_strategy: eris::hub::DelegationStrategy::Gauges {
                amp_gauges: "amp_gauges".to_string(),
                emp_gauges: Some("emp_gauges".to_string()),
                amp_factor_bps: 5000,
                min_delegation_bps: 100,
                max_delegation_bps: 2500,
                validator_count: 5,
            }
        }
    );

    let res: StateResponse = query_helper(deps.as_ref(), QueryMsg::State {});
    assert_eq!(
        res,
        StateResponse {
            total_ustake: Uint128::zero(),
            total_uluna: Uint128::zero(),
            exchange_rate: Decimal::one(),
            unlocked_coins: vec![],
            unbonding: Uint128::zero(),
            available: Uint128::zero(),
            tvl_uluna: Uint128::zero(),
        },
    );

    let res: PendingBatch = query_helper(deps.as_ref(), QueryMsg::PendingBatch {});
    assert_eq!(
        res,
        PendingBatch {
            id: 1,
            ustake_to_burn: Uint128::zero(),
            est_unbond_start_time: 269200, // 10,000 + 259,200
        },
    );
}

#[test]
fn bonding() {
    let mut deps = setup_test();

    deps.querier.set_bank_balances(&[coin(1000100, CONTRACT_DENOM)]);

    // Bond when no delegation has been made
    // In this case, the full deposit simply goes to the first validator
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("user_1", &[Coin::new(1000000, CONTRACT_DENOM)]),
        ExecuteMsg::Bond {
            receiver: None,
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 3);
    assert_eq!(res.messages[0], SubMsg::new(Delegation::new("alice", 1000000).to_cosmos_msg()));
    assert_eq!(
        res.messages[1],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: STAKE_DENOM.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: "user_1".to_string(),
                amount: Uint128::new(1000000)
            })
            .unwrap(),
            funds: vec![]
        }))
    );

    assert_eq!(res.messages[2], check_received_coin(100));

    deps.querier.set_bank_balances(&[coin(12345 + 222, CONTRACT_DENOM)]);

    // Bond when there are existing delegations, and Token:Stake exchange rate is >1
    // Previously user 1 delegated 1,000,000 utoken. We assume we have accumulated 2.5% yield at 1025000 staked
    deps.querier.set_staking_delegations(&[
        Delegation::new("alice", 341667),
        Delegation::new("bob", 341667),
        Delegation::new("charlie", 341666),
    ]);
    deps.querier.set_cw20_total_supply(STAKE_DENOM, 1000000);

    // Charlie has the smallest amount of delegation, so the full deposit goes to him
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("user_2", &[Coin::new(12345, CONTRACT_DENOM)]),
        ExecuteMsg::Bond {
            receiver: Some("user_3".to_string()),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 3);
    assert_eq!(res.messages[0], SubMsg::new(Delegation::new("charlie", 12345).to_cosmos_msg()));
    assert_eq!(
        res.messages[1],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: STAKE_DENOM.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: "user_3".to_string(),
                amount: Uint128::new(12043)
            })
            .unwrap(),
            funds: vec![]
        }))
    );

    assert_eq!(res.messages[2], check_received_coin(222));

    // Check the state after bonding
    deps.querier.set_staking_delegations(&[
        Delegation::new("alice", 341667),
        Delegation::new("bob", 341667),
        Delegation::new("charlie", 354011),
    ]);
    deps.querier.set_cw20_total_supply(STAKE_DENOM, 1012043);

    let res: StateResponse = query_helper(deps.as_ref(), QueryMsg::State {});
    assert_eq!(
        res,
        StateResponse {
            total_ustake: Uint128::new(1012043),
            total_uluna: Uint128::new(1037345),
            exchange_rate: Decimal::from_ratio(1037345u128, 1012043u128),
            unlocked_coins: vec![],
            unbonding: Uint128::zero(),
            available: Uint128::new(12567),
            tvl_uluna: Uint128::new(1037345 + 12567),
        }
    );
}

#[test]
fn donating() {
    let mut deps = setup_test();

    deps.querier.set_bank_balances(&[coin(1000100, CONTRACT_DENOM)]);
    // Bond when no delegation has been made
    // In this case, the full deposit simply goes to the first validator
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("user_1", &[Coin::new(1000000, CONTRACT_DENOM)]),
        ExecuteMsg::Bond {
            receiver: None,
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 3);
    assert_eq!(res.messages[0], SubMsg::new(Delegation::new("alice", 1000000).to_cosmos_msg()));
    assert_eq!(
        res.messages[1],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: STAKE_DENOM.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: "user_1".to_string(),
                amount: Uint128::new(1000000)
            })
            .unwrap(),
            funds: vec![]
        }))
    );
    assert_eq!(res.messages[2], check_received_coin(100));

    deps.querier.set_bank_balances(&[coin(100, CONTRACT_DENOM)]);
    // Bond when there are existing delegations, and Token:Stake exchange rate is >1
    // Previously user 1 delegated 1,000,000 utoken. We assume we have accumulated 2.5% yield at 1025000 staked
    deps.querier.set_staking_delegations(&[
        Delegation::new("alice", 341667),
        Delegation::new("bob", 341667),
        Delegation::new("charlie", 341666),
    ]);
    deps.querier.set_cw20_total_supply(STAKE_DENOM, 1000000);

    let res: StateResponse = query_helper(deps.as_ref(), QueryMsg::State {});
    assert_eq!(
        res,
        StateResponse {
            total_ustake: Uint128::new(1000000),
            total_uluna: Uint128::new(1025000),
            exchange_rate: Decimal::from_ratio(1025000u128, 1000000u128),
            unlocked_coins: vec![],
            unbonding: Uint128::zero(),
            available: Uint128::new(100),
            tvl_uluna: Uint128::new(1025100),
        }
    );

    deps.querier.set_bank_balances(&[coin(100 + 12345, CONTRACT_DENOM)]);
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("user_2", &[Coin::new(12345, CONTRACT_DENOM)]),
        ExecuteMsg::Donate {},
    )
    .unwrap_err();
    assert_eq!(res, ContractError::DonationsDisabled {});

    // enable donations
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::UpdateConfig {
            protocol_fee_contract: None,
            protocol_reward_fee: None,
            allow_donations: Some(true),
            delegation_strategy: None,
            vote_operator: None,
        },
    )
    .unwrap();

    // Charlie has the smallest amount of delegation, so the full deposit goes to him
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("user_2", &[Coin::new(12345, CONTRACT_DENOM)]),
        ExecuteMsg::Donate {},
    )
    .unwrap();

    assert_eq!(res.messages.len(), 2);
    assert_eq!(res.messages[0], SubMsg::new(Delegation::new("charlie", 12345).to_cosmos_msg()));
    assert_eq!(res.messages[1], check_received_coin(100));

    deps.querier.set_bank_balances(&[coin(100, CONTRACT_DENOM)]);
    // Check the state after bonding
    deps.querier.set_staking_delegations(&[
        Delegation::new("alice", 341667),
        Delegation::new("bob", 341667),
        Delegation::new("charlie", 354011),
    ]);

    // nothing has been minted -> ustake stays the same, only utoken and exchange rate is changing.
    let res: StateResponse = query_helper(deps.as_ref(), QueryMsg::State {});
    assert_eq!(
        res,
        StateResponse {
            total_ustake: Uint128::new(1000000),
            total_uluna: Uint128::new(1037345),
            exchange_rate: Decimal::from_ratio(1037345u128, 1000000u128),
            unlocked_coins: vec![],
            unbonding: Uint128::zero(),
            available: Uint128::new(100),
            tvl_uluna: Uint128::new(1037345 + 100),
        }
    );
}

#[test]
fn harvesting() {
    let mut deps = setup_test();

    // Assume users have bonded a total of 1,000,000 utoken and minted the same amount of ustake
    deps.querier.set_staking_delegations(&[
        Delegation::new("alice", 341667),
        Delegation::new("bob", 341667),
        Delegation::new("charlie", 341666),
    ]);
    deps.querier.set_cw20_total_supply(STAKE_DENOM, 1000000);

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::Harvest {},
    )
    .unwrap();

    assert_eq!(res.messages.len(), 5);
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward {
            validator: "alice".to_string(),
        }))
    );
    assert_eq!(
        res.messages[1],
        SubMsg::new(CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward {
            validator: "bob".to_string(),
        }))
    );
    assert_eq!(
        res.messages[2],
        SubMsg::new(CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward {
            validator: "charlie".to_string(),
        }))
    );
    assert_eq!(res.messages[3], check_received_coin(0));
    assert_eq!(
        res.messages[4],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: MOCK_CONTRACT_ADDR.to_string(),
            msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::Reinvest {})).unwrap(),
            funds: vec![]
        }))
    );
}

#[test]
fn registering_unlocked_coins() {
    let mut deps = setup_test();

    let state = State::default();

    deps.querier.set_bank_balances(&[coin(100 + 123, CONTRACT_DENOM)]);

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::Callback(CallbackMsg::CheckReceivedCoin {
            snapshot: coin(100, CONTRACT_DENOM),
        }),
    )
    .unwrap();

    assert_eq!(
        res.events,
        vec![Event::new("erishub/received")
            .add_attribute("received_coin", 123.to_string() + CONTRACT_DENOM)]
    );

    // Unlocked coins in contract state should have been updated
    let unlocked_coins = state.unlocked_coins.load(deps.as_ref().storage).unwrap();
    assert_eq!(unlocked_coins, vec![Coin::new(123, CONTRACT_DENOM),]);
}

#[test]
fn reinvesting() {
    let mut deps = setup_test();
    let state = State::default();

    deps.querier.set_staking_delegations(&[
        Delegation::new("alice", 333334),
        Delegation::new("bob", 333333),
        Delegation::new("charlie", 333333),
    ]);

    // After the swaps, `unlocked_coins` should contain only utoken and unknown denoms
    state
        .unlocked_coins
        .save(
            deps.as_mut().storage,
            &vec![
                Coin::new(234, CONTRACT_DENOM),
                Coin::new(
                    69420,
                    "ibc/0471F1C4E7AFD3F07702BEF6DC365268D64570F7C1FDC98EA6098DD6DE59817B",
                ),
            ],
        )
        .unwrap();

    // Bob has the smallest amount of delegations, so all proceeds go to him
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::Callback(CallbackMsg::Reinvest {}),
    )
    .unwrap();

    assert_eq!(res.messages.len(), 2);

    let total = Uint128::from(234u128);
    let fee =
        Decimal::from_ratio(1u128, 100u128).checked_mul_uint(total).expect("expects fee result");
    let delegated = total.saturating_sub(fee);

    assert_eq!(
        res.messages[0],
        SubMsg::new(Delegation::new("bob", delegated.u128()).to_cosmos_msg())
    );

    assert_eq!(
        res.messages[1],
        SubMsg::new(SendFee::new(Addr::unchecked("fee"), fee.u128()).to_cosmos_msg())
    );

    // Storage should have been updated
    let unlocked_coins = state.unlocked_coins.load(deps.as_ref().storage).unwrap();
    assert_eq!(
        unlocked_coins,
        vec![Coin::new(
            69420,
            "ibc/0471F1C4E7AFD3F07702BEF6DC365268D64570F7C1FDC98EA6098DD6DE59817B"
        )],
    );
}

#[test]
fn queuing_unbond() {
    let mut deps = setup_test();
    let state = State::default();

    // Only Stake token is accepted for unbonding requests
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("random_token", &[]),
        ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
            sender: "hacker".to_string(),
            amount: Uint128::new(69420),
            msg: to_binary(&ReceiveMsg::QueueUnbond {
                receiver: None,
            })
            .unwrap(),
        }),
    )
    .unwrap_err();

    assert_eq!(err, ContractError::ExpectingStakeToken("random_token".into()));

    // User 1 creates an unbonding request before `est_unbond_start_time` is reached. The unbond
    // request is saved, but not the pending batch is not submitted for unbonding
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(12345), // est_unbond_start_time = 269200
        mock_info(STAKE_DENOM, &[]),
        ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
            sender: "user_1".to_string(),
            amount: Uint128::new(23456),
            msg: to_binary(&ReceiveMsg::QueueUnbond {
                receiver: None,
            })
            .unwrap(),
        }),
    )
    .unwrap();

    assert_eq!(res.messages.len(), 0);

    // User 2 creates an unbonding request after `est_unbond_start_time` is reached. The unbond
    // request is saved, and the pending is automatically submitted for unbonding
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(269201), // est_unbond_start_time = 269200
        mock_info(STAKE_DENOM, &[]),
        ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
            sender: "user_2".to_string(),
            amount: Uint128::new(69420),
            msg: to_binary(&ReceiveMsg::QueueUnbond {
                receiver: Some("user_3".to_string()),
            })
            .unwrap(),
        }),
    )
    .unwrap();

    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: MOCK_CONTRACT_ADDR.to_string(),
            msg: to_binary(&ExecuteMsg::SubmitBatch {}).unwrap(),
            funds: vec![]
        }))
    );

    // The users' unbonding requests should have been saved
    let ubr1 = state
        .unbond_requests
        .load(deps.as_ref().storage, (1u64, &Addr::unchecked("user_1")))
        .unwrap();
    let ubr2 = state
        .unbond_requests
        .load(deps.as_ref().storage, (1u64, &Addr::unchecked("user_3")))
        .unwrap();

    assert_eq!(
        ubr1,
        UnbondRequest {
            id: 1,
            user: Addr::unchecked("user_1"),
            shares: Uint128::new(23456)
        }
    );
    assert_eq!(
        ubr2,
        UnbondRequest {
            id: 1,
            user: Addr::unchecked("user_3"),
            shares: Uint128::new(69420)
        }
    );

    // Pending batch should have been updated
    let pending_batch = state.pending_batch.load(deps.as_ref().storage).unwrap();
    assert_eq!(
        pending_batch,
        PendingBatch {
            id: 1,
            ustake_to_burn: Uint128::new(92876), // 23,456 + 69,420
            est_unbond_start_time: 269200
        }
    );
}

#[test]
fn submitting_batch() {
    let mut deps = setup_test();
    let state = State::default();

    // utoken bonded: 1,037,345
    // ustake supply: 1,012,043
    // utoken per ustake: 1.025
    deps.querier.set_staking_delegations(&[
        Delegation::new("alice", 345782),
        Delegation::new("bob", 345782),
        Delegation::new("charlie", 345781),
    ]);
    deps.querier.set_cw20_total_supply(STAKE_DENOM, 1012043);

    // We continue from the contract state at the end of the last test
    let unbond_requests = vec![
        UnbondRequest {
            id: 1,
            user: Addr::unchecked("user_1"),
            shares: Uint128::new(23456),
        },
        UnbondRequest {
            id: 1,
            user: Addr::unchecked("user_3"),
            shares: Uint128::new(69420),
        },
    ];

    for unbond_request in &unbond_requests {
        state
            .unbond_requests
            .save(
                deps.as_mut().storage,
                (unbond_request.id, &Addr::unchecked(unbond_request.user.clone())),
                unbond_request,
            )
            .unwrap();
    }

    state
        .pending_batch
        .save(
            deps.as_mut().storage,
            &PendingBatch {
                id: 1,
                ustake_to_burn: Uint128::new(92876), // 23,456 + 69,420
                est_unbond_start_time: 269200,
            },
        )
        .unwrap();

    // Anyone can invoke `submit_batch`. Here we continue from the previous test and assume it is
    // invoked automatically as user 2 submits the unbonding request
    //
    // ustake to burn: 23,456 + 69,420 = 92,876
    // utoken to unbond: 1,037,345 * 92,876 / 1,012,043 = 95,197
    //
    // Target: (1,037,345 - 95,197) / 3 = 314,049
    // Remainer: 1
    // Alice:   345,782 - (314,049 + 1) = 31,732
    // Bob:     345,782 - (314,049 + 0) = 31,733
    // Charlie: 345,781 - (314,049 + 0) = 31,732
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(269201),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::SubmitBatch {},
    )
    .unwrap();

    assert_eq!(res.messages.len(), 5);
    assert_eq!(res.messages[0], SubMsg::new(Undelegation::new("alice", 31732).to_cosmos_msg()));
    assert_eq!(res.messages[1], SubMsg::new(Undelegation::new("bob", 31733).to_cosmos_msg()));
    assert_eq!(res.messages[2], SubMsg::new(Undelegation::new("charlie", 31732).to_cosmos_msg()));
    assert_eq!(
        res.messages[3],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: STAKE_DENOM.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Burn {
                amount: Uint128::new(92876)
            })
            .unwrap(),
            funds: vec![]
        }))
    );
    assert_eq!(res.messages[4], check_received_coin(0));

    // A new pending batch should have been created
    let pending_batch = state.pending_batch.load(deps.as_ref().storage).unwrap();
    assert_eq!(
        pending_batch,
        PendingBatch {
            id: 2,
            ustake_to_burn: Uint128::zero(),
            est_unbond_start_time: 528401 // 269,201 + 259,200
        }
    );

    // Previous batch should have been updated
    let previous_batch = state.previous_batches.load(deps.as_ref().storage, 1u64).unwrap();
    assert_eq!(
        previous_batch,
        Batch {
            id: 1,
            reconciled: false,
            total_shares: Uint128::new(92876),
            uluna_unclaimed: Uint128::new(95197),
            est_unbond_end_time: 2083601 // 269,201 + 1,814,400
        }
    );

    let res: StateResponse = query_helper_env(deps.as_ref(), QueryMsg::State {}, 2083600);
    assert_eq!(
        res,
        StateResponse {
            total_ustake: Uint128::from(1012043u128),
            total_uluna: Uint128::from(1037345u128),
            exchange_rate: Decimal::from_ratio(1037345u128, 1012043u128),
            unlocked_coins: vec![],
            unbonding: Uint128::from(95197u128),
            available: Uint128::zero(),
            tvl_uluna: Uint128::from(95197u128 + 1037345u128),
        },
    );
}

#[test]
fn reconciling() {
    let mut deps = setup_test();
    let state = State::default();

    let previous_batches = vec![
        Batch {
            id: 1,
            reconciled: true,
            total_shares: Uint128::new(92876),
            uluna_unclaimed: Uint128::new(95197), // 1.025 Token per Stake
            est_unbond_end_time: 10000,
        },
        Batch {
            id: 2,
            reconciled: false,
            total_shares: Uint128::new(1345),
            uluna_unclaimed: Uint128::new(1385), // 1.030 Token per Stake
            est_unbond_end_time: 20000,
        },
        Batch {
            id: 3,
            reconciled: false,
            total_shares: Uint128::new(1456),
            uluna_unclaimed: Uint128::new(1506), // 1.035 Token per Stake
            est_unbond_end_time: 30000,
        },
        Batch {
            id: 4,
            reconciled: false,
            total_shares: Uint128::new(1567),
            uluna_unclaimed: Uint128::new(1629), // 1.040 Token per Stake
            est_unbond_end_time: 40000,          // not yet finished unbonding, ignored
        },
    ];

    for previous_batch in &previous_batches {
        state
            .previous_batches
            .save(deps.as_mut().storage, previous_batch.id, previous_batch)
            .unwrap();
    }

    state
        .unlocked_coins
        .save(
            deps.as_mut().storage,
            &vec![
                Coin::new(10000, CONTRACT_DENOM),
                Coin::new(234, "ukrw"),
                Coin::new(345, "uusd"),
                Coin::new(
                    69420,
                    "ibc/0471F1C4E7AFD3F07702BEF6DC365268D64570F7C1FDC98EA6098DD6DE59817B",
                ),
            ],
        )
        .unwrap();

    deps.querier.set_bank_balances(&[
        Coin::new(12345, CONTRACT_DENOM),
        Coin::new(234, "ukrw"),
        Coin::new(345, "uusd"),
        Coin::new(69420, "ibc/0471F1C4E7AFD3F07702BEF6DC365268D64570F7C1FDC98EA6098DD6DE59817B"),
    ]);

    execute(
        deps.as_mut(),
        mock_env_at_timestamp(35000),
        mock_info("worker", &[]),
        ExecuteMsg::Reconcile {},
    )
    .unwrap();

    // Expected received: batch 2 + batch 3 = 1385 + 1506 = 2891
    // Expected unlocked: 10000
    // Expected: 12891
    // Actual: 12345
    // Shortfall: 12891 - 12345 = 456
    //
    // utoken per batch: 546 / 2 = 273
    // remainder: 0
    // batch 2: 1385 - 273 = 1112
    // batch 3: 1506 - 273 = 1233
    let batch = state.previous_batches.load(deps.as_ref().storage, 2u64).unwrap();
    assert_eq!(
        batch,
        Batch {
            id: 2,
            reconciled: true,
            total_shares: Uint128::new(1345),
            uluna_unclaimed: Uint128::new(1112), // 1385 - 273
            est_unbond_end_time: 20000,
        }
    );

    let batch = state.previous_batches.load(deps.as_ref().storage, 3u64).unwrap();
    assert_eq!(
        batch,
        Batch {
            id: 3,
            reconciled: true,
            total_shares: Uint128::new(1456),
            uluna_unclaimed: Uint128::new(1233), // 1506 - 273
            est_unbond_end_time: 30000,
        }
    );

    // Batches 1 and 4 should not have changed
    let batch = state.previous_batches.load(deps.as_ref().storage, 1u64).unwrap();
    assert_eq!(batch, previous_batches[0]);

    let batch = state.previous_batches.load(deps.as_ref().storage, 4u64).unwrap();
    assert_eq!(batch, previous_batches[3]);
}

#[test]
fn reconciling_even_when_everything_ok() {
    let mut deps = setup_test();
    let state = State::default();

    let previous_batches = vec![
        Batch {
            id: 1,
            reconciled: true,
            total_shares: Uint128::new(100000),
            uluna_unclaimed: Uint128::new(100000),
            est_unbond_end_time: 10000,
        },
        Batch {
            id: 2,
            reconciled: false,
            total_shares: Uint128::new(1000),
            uluna_unclaimed: Uint128::new(1000),
            est_unbond_end_time: 20000,
        },
        Batch {
            id: 3,
            reconciled: false,
            total_shares: Uint128::new(1500),
            uluna_unclaimed: Uint128::new(1500),
            est_unbond_end_time: 30000,
        },
        Batch {
            id: 4,
            reconciled: false,
            total_shares: Uint128::new(1500),
            uluna_unclaimed: Uint128::new(1500),
            est_unbond_end_time: 40000, // not yet finished unbonding, ignored
        },
    ];

    for previous_batch in &previous_batches {
        state
            .previous_batches
            .save(deps.as_mut().storage, previous_batch.id, previous_batch)
            .unwrap();
    }

    state
        .unlocked_coins
        .save(deps.as_mut().storage, &vec![Coin::new(1000, CONTRACT_DENOM)])
        .unwrap();

    deps.querier.set_bank_balances(&[Coin::new(3500, CONTRACT_DENOM)]);

    execute(
        deps.as_mut(),
        mock_env_at_timestamp(35000),
        mock_info("worker", &[]),
        ExecuteMsg::Reconcile {},
    )
    .unwrap();

    let batch = state.previous_batches.load(deps.as_ref().storage, 2u64).unwrap();
    assert_eq!(
        batch,
        Batch {
            id: 2,
            reconciled: true,
            total_shares: Uint128::new(1000),
            uluna_unclaimed: Uint128::new(1000),
            est_unbond_end_time: 20000,
        }
    );

    let batch = state.previous_batches.load(deps.as_ref().storage, 3u64).unwrap();
    assert_eq!(
        batch,
        Batch {
            id: 3,
            reconciled: true,
            total_shares: Uint128::new(1500),
            uluna_unclaimed: Uint128::new(1500),
            est_unbond_end_time: 30000,
        }
    );

    // Batches 1 and 4 should not have changed
    let batch = state.previous_batches.load(deps.as_ref().storage, 1u64).unwrap();
    assert_eq!(batch, previous_batches[0]);

    let batch = state.previous_batches.load(deps.as_ref().storage, 4u64).unwrap();
    assert_eq!(batch, previous_batches[3]);
}

#[test]
fn withdrawing_unbonded() {
    let mut deps = setup_test();
    let state = State::default();

    // We simulate a most general case:
    // - batches 1 and 2 have finished unbonding
    // - batch 3 have been submitted for unbonding but have not finished
    // - batch 4 is still pending
    let unbond_requests = vec![
        UnbondRequest {
            id: 1,
            user: Addr::unchecked("user_1"),
            shares: Uint128::new(23456),
        },
        UnbondRequest {
            id: 1,
            user: Addr::unchecked("user_3"),
            shares: Uint128::new(69420),
        },
        UnbondRequest {
            id: 2,
            user: Addr::unchecked("user_1"),
            shares: Uint128::new(34567),
        },
        UnbondRequest {
            id: 3,
            user: Addr::unchecked("user_1"),
            shares: Uint128::new(45678),
        },
        UnbondRequest {
            id: 4,
            user: Addr::unchecked("user_1"),
            shares: Uint128::new(56789),
        },
    ];

    for unbond_request in &unbond_requests {
        state
            .unbond_requests
            .save(
                deps.as_mut().storage,
                (unbond_request.id, &Addr::unchecked(unbond_request.user.clone())),
                unbond_request,
            )
            .unwrap();
    }

    let previous_batches = vec![
        Batch {
            id: 1,
            reconciled: true,
            total_shares: Uint128::new(92876),
            uluna_unclaimed: Uint128::new(95197), // 1.025 Token per Stake
            est_unbond_end_time: 10000,
        },
        Batch {
            id: 2,
            reconciled: true,
            total_shares: Uint128::new(34567),
            uluna_unclaimed: Uint128::new(35604), // 1.030 Token per Stake
            est_unbond_end_time: 20000,
        },
        Batch {
            id: 3,
            reconciled: false, // finished unbonding, but not reconciled; ignored
            total_shares: Uint128::new(45678),
            uluna_unclaimed: Uint128::new(47276), // 1.035 Token per Stake
            est_unbond_end_time: 20000,
        },
        Batch {
            id: 4,
            reconciled: true,
            total_shares: Uint128::new(56789),
            uluna_unclaimed: Uint128::new(59060), // 1.040 Token per Stake
            est_unbond_end_time: 30000, // reconciled, but not yet finished unbonding; ignored
        },
    ];

    for previous_batch in &previous_batches {
        state
            .previous_batches
            .save(deps.as_mut().storage, previous_batch.id, previous_batch)
            .unwrap();
    }

    state
        .pending_batch
        .save(
            deps.as_mut().storage,
            &PendingBatch {
                id: 4,
                ustake_to_burn: Uint128::new(56789),
                est_unbond_start_time: 100000,
            },
        )
        .unwrap();

    // Attempt to withdraw before any batch has completed unbonding. Should error
    let err = execute(
        deps.as_mut(),
        mock_env_at_timestamp(5000),
        mock_info("user_1", &[]),
        ExecuteMsg::WithdrawUnbonded {
            receiver: None,
        },
    )
    .unwrap_err();

    assert_eq!(err, ContractError::CantBeZero("withdrawable amount".into()));

    // Attempt to withdraw once batches 1 and 2 have finished unbonding, but 3 has not yet
    //
    // Withdrawable from batch 1: 95,197 * 23,456 / 92,876 = 24,042
    // Withdrawable from batch 2: 35,604
    // Total withdrawable: 24,042 + 35,604 = 59,646
    //
    // Batch 1 should be updated:
    // Total shares: 92,876 - 23,456 = 69,420
    // Unclaimed utoken: 95,197 - 24,042 = 71,155
    //
    // Batch 2 is completely withdrawn, should be purged from storage
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(25000),
        mock_info("user_1", &[]),
        ExecuteMsg::WithdrawUnbonded {
            receiver: None,
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "user_1".to_string(),
            amount: vec![Coin::new(59646, CONTRACT_DENOM)]
        }))
    );

    // Previous batches should have been updated
    let batch = state.previous_batches.load(deps.as_ref().storage, 1u64).unwrap();
    assert_eq!(
        batch,
        Batch {
            id: 1,
            reconciled: true,
            total_shares: Uint128::new(69420),
            uluna_unclaimed: Uint128::new(71155),
            est_unbond_end_time: 10000,
        }
    );

    let err = state.previous_batches.load(deps.as_ref().storage, 2u64).unwrap_err();
    assert_eq!(
        err,
        StdError::NotFound {
            kind: "eris::hub::Batch".to_string()
        }
    );

    // User 1's unbond requests in batches 1 and 2 should have been deleted
    let err1 = state
        .unbond_requests
        .load(deps.as_ref().storage, (1u64, &Addr::unchecked("user_1")))
        .unwrap_err();
    let err2 = state
        .unbond_requests
        .load(deps.as_ref().storage, (1u64, &Addr::unchecked("user_1")))
        .unwrap_err();

    assert_eq!(
        err1,
        StdError::NotFound {
            kind: "eris::hub::UnbondRequest".to_string()
        }
    );
    assert_eq!(
        err2,
        StdError::NotFound {
            kind: "eris::hub::UnbondRequest".to_string()
        }
    );

    // User 3 attempt to withdraw; also specifying a receiver
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(25000),
        mock_info("user_3", &[]),
        ExecuteMsg::WithdrawUnbonded {
            receiver: Some("user_2".to_string()),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "user_2".to_string(),
            amount: vec![Coin::new(71155, CONTRACT_DENOM)]
        }))
    );

    // Batch 1 and user 2's unbonding request should have been purged from storage
    let err = state.previous_batches.load(deps.as_ref().storage, 1u64).unwrap_err();
    assert_eq!(
        err,
        StdError::NotFound {
            kind: "eris::hub::Batch".to_string()
        }
    );

    let err = state
        .unbond_requests
        .load(deps.as_ref().storage, (1u64, &Addr::unchecked("user_3")))
        .unwrap_err();

    assert_eq!(
        err,
        StdError::NotFound {
            kind: "eris::hub::UnbondRequest".to_string()
        }
    );
}

#[test]
fn adding_validator() {
    let mut deps = setup_test();
    let state = State::default();

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake", &[]),
        ExecuteMsg::AddValidator {
            validator: "dave".to_string(),
        },
    )
    .unwrap_err();

    assert_eq!(err, ContractError::Unauthorized {});

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::AddValidator {
            validator: "alice".to_string(),
        },
    )
    .unwrap_err();

    assert_eq!(err, ContractError::ValidatorAlreadyWhitelisted("alice".into()));

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::AddValidator {
            validator: "dave".to_string(),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 0);

    let validators = state.validators.load(deps.as_ref().storage).unwrap();
    assert_eq!(
        validators,
        vec![
            String::from("alice"),
            String::from("bob"),
            String::from("charlie"),
            String::from("dave")
        ],
    );
}

#[test]
fn removing_validator() {
    let mut deps = setup_test();
    let state = State::default();

    deps.querier.set_staking_delegations(&[
        Delegation::new("alice", 341667),
        Delegation::new("bob", 341667),
        Delegation::new("charlie", 341666),
    ]);

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake", &[]),
        ExecuteMsg::RemoveValidator {
            validator: "charlie".to_string(),
        },
    )
    .unwrap_err();

    assert_eq!(err, ContractError::Unauthorized {});

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::RemoveValidator {
            validator: "dave".to_string(),
        },
    )
    .unwrap_err();

    assert_eq!(err, ContractError::ValidatorNotWhitelisted("dave".into()));

    // Target: (341667 + 341667 + 341666) / 2 = 512500
    // Remainder: 0
    // Alice:   512500 + 0 - 341667 = 170833
    // Bob:     512500 + 0 - 341667 = 170833
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::RemoveValidator {
            validator: "charlie".to_string(),
        },
    )
    .unwrap();

    // Gauges does not redelegate through removal
    assert_eq!(res.messages.len(), 0);

    let validators = state.validators.load(deps.as_ref().storage).unwrap();
    assert_eq!(validators, vec![String::from("alice"), String::from("bob")],);
}

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

    assert_eq!(err, ContractError::Unauthorized {});

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
    let new_owner = state.new_owner.load(deps.as_ref().storage).unwrap();
    assert_eq!(new_owner, Addr::unchecked("jake"));

    // Check dropping ownership proposal
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("pumpkin", &[]),
        ExecuteMsg::DropOwnershipProposal {},
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::DropOwnershipProposal {},
    )
    .unwrap();
    assert_eq!(res.messages.len(), 0);

    let owner = state.owner.load(deps.as_ref().storage).unwrap();
    assert_eq!(owner, Addr::unchecked("owner"));
    let new_owner = state.new_owner.may_load(deps.as_ref().storage).unwrap();
    assert_eq!(new_owner, None);

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
    let new_owner = state.new_owner.load(deps.as_ref().storage).unwrap();
    assert_eq!(new_owner, Addr::unchecked("jake"));

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("pumpkin", &[]),
        ExecuteMsg::AcceptOwnership {},
    )
    .unwrap_err();

    assert_eq!(err, ContractError::UnauthorizedSenderNotNewOwner {});

    let res =
        execute(deps.as_mut(), mock_env(), mock_info("jake", &[]), ExecuteMsg::AcceptOwnership {})
            .unwrap();

    assert_eq!(res.messages.len(), 0);

    let owner = state.owner.load(deps.as_ref().storage).unwrap();
    assert_eq!(owner, Addr::unchecked("jake"));
}

//--------------------------------------------------------------------------------------------------
// Fee Config
//--------------------------------------------------------------------------------------------------

#[test]
fn update_fee() {
    let mut deps = setup_test();
    let state = State::default();

    let config = state.fee_config.load(deps.as_ref().storage).unwrap();
    assert_eq!(
        config,
        FeeConfig {
            protocol_fee_contract: Addr::unchecked("fee"),
            protocol_reward_fee: Decimal::from_ratio(1u128, 100u128)
        }
    );

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake", &[]),
        ExecuteMsg::UpdateConfig {
            protocol_fee_contract: None,
            protocol_reward_fee: Some(Decimal::from_ratio(11u128, 100u128)),
            delegation_strategy: None,
            allow_donations: None,
            vote_operator: None,
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::UpdateConfig {
            protocol_fee_contract: None,
            protocol_reward_fee: Some(Decimal::from_ratio(11u128, 100u128)),
            delegation_strategy: None,
            allow_donations: None,
            vote_operator: None,
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::ProtocolRewardFeeTooHigh {});

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::UpdateConfig {
            protocol_fee_contract: Some("fee-new".to_string()),
            protocol_reward_fee: Some(Decimal::from_ratio(10u128, 100u128)),
            delegation_strategy: None,
            allow_donations: None,
            vote_operator: None,
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 0);

    let config = state.fee_config.load(deps.as_ref().storage).unwrap();
    assert_eq!(
        config,
        FeeConfig {
            protocol_fee_contract: Addr::unchecked("fee-new"),
            protocol_reward_fee: Decimal::from_ratio(10u128, 100u128)
        }
    );
}

//--------------------------------------------------------------------------------------------------
// Queries
//--------------------------------------------------------------------------------------------------

#[test]
fn querying_previous_batches() {
    let mut deps = mock_dependencies();

    let batches = vec![
        Batch {
            id: 1,
            reconciled: false,
            total_shares: Uint128::new(123),
            uluna_unclaimed: Uint128::new(678),
            est_unbond_end_time: 10000,
        },
        Batch {
            id: 2,
            reconciled: true,
            total_shares: Uint128::new(234),
            uluna_unclaimed: Uint128::new(789),
            est_unbond_end_time: 15000,
        },
        Batch {
            id: 3,
            reconciled: false,
            total_shares: Uint128::new(345),
            uluna_unclaimed: Uint128::new(890),
            est_unbond_end_time: 20000,
        },
        Batch {
            id: 4,
            reconciled: true,
            total_shares: Uint128::new(456),
            uluna_unclaimed: Uint128::new(999),
            est_unbond_end_time: 25000,
        },
    ];

    let state = State::default();
    for batch in &batches {
        state.previous_batches.save(deps.as_mut().storage, batch.id, batch).unwrap();
    }

    // Querying a single batch
    let res: Batch = query_helper(deps.as_ref(), QueryMsg::PreviousBatch(1));
    assert_eq!(res, batches[0].clone());

    let res: Batch = query_helper(deps.as_ref(), QueryMsg::PreviousBatch(2));
    assert_eq!(res, batches[1].clone());

    // Query multiple batches
    let res: Vec<Batch> = query_helper(
        deps.as_ref(),
        QueryMsg::PreviousBatches {
            start_after: None,
            limit: None,
        },
    );
    assert_eq!(res, batches);

    let res: Vec<Batch> = query_helper(
        deps.as_ref(),
        QueryMsg::PreviousBatches {
            start_after: Some(1),
            limit: None,
        },
    );
    assert_eq!(res, vec![batches[1].clone(), batches[2].clone(), batches[3].clone()]);

    let res: Vec<Batch> = query_helper(
        deps.as_ref(),
        QueryMsg::PreviousBatches {
            start_after: Some(4),
            limit: None,
        },
    );
    assert_eq!(res, vec![]);

    // Query multiple batches, indexed by whether it has been reconciled
    let res = state
        .previous_batches
        .idx
        .reconciled
        .prefix(true.into())
        .range(deps.as_ref().storage, None, None, Order::Ascending)
        .map(|item| {
            let (_, v) = item.unwrap();
            v
        })
        .collect::<Vec<_>>();

    assert_eq!(res, vec![batches[1].clone(), batches[3].clone()]);

    let res = state
        .previous_batches
        .idx
        .reconciled
        .prefix(false.into())
        .range(deps.as_ref().storage, None, None, Order::Ascending)
        .map(|item| {
            let (_, v) = item.unwrap();
            v
        })
        .collect::<Vec<_>>();

    assert_eq!(res, vec![batches[0].clone(), batches[2].clone()]);
}

#[test]
fn querying_unbond_requests() {
    let mut deps = mock_dependencies();
    let state = State::default();

    let unbond_requests = vec![
        UnbondRequest {
            id: 1,
            user: Addr::unchecked("alice"),
            shares: Uint128::new(123),
        },
        UnbondRequest {
            id: 1,
            user: Addr::unchecked("bob"),
            shares: Uint128::new(234),
        },
        UnbondRequest {
            id: 1,
            user: Addr::unchecked("charlie"),
            shares: Uint128::new(345),
        },
        UnbondRequest {
            id: 2,
            user: Addr::unchecked("alice"),
            shares: Uint128::new(456),
        },
    ];

    for unbond_request in &unbond_requests {
        state
            .unbond_requests
            .save(
                deps.as_mut().storage,
                (unbond_request.id, &Addr::unchecked(unbond_request.user.clone())),
                unbond_request,
            )
            .unwrap();
    }

    let res: Vec<UnbondRequestsByBatchResponseItem> = query_helper(
        deps.as_ref(),
        QueryMsg::UnbondRequestsByBatch {
            id: 1,
            start_after: None,
            limit: None,
        },
    );
    assert_eq!(
        res,
        vec![
            unbond_requests[0].clone().into(),
            unbond_requests[1].clone().into(),
            unbond_requests[2].clone().into(),
        ]
    );

    let res: Vec<UnbondRequestsByBatchResponseItem> = query_helper(
        deps.as_ref(),
        QueryMsg::UnbondRequestsByBatch {
            id: 2,
            start_after: None,
            limit: None,
        },
    );
    assert_eq!(res, vec![unbond_requests[3].clone().into()]);

    let res: Vec<UnbondRequestsByUserResponseItem> = query_helper(
        deps.as_ref(),
        QueryMsg::UnbondRequestsByUser {
            user: "alice".to_string(),
            start_after: None,
            limit: None,
        },
    );
    assert_eq!(res, vec![unbond_requests[0].clone().into(), unbond_requests[3].clone().into(),]);

    let res: Vec<UnbondRequestsByUserResponseItem> = query_helper(
        deps.as_ref(),
        QueryMsg::UnbondRequestsByUser {
            user: "alice".to_string(),
            start_after: Some(1u64),
            limit: None,
        },
    );
    assert_eq!(res, vec![unbond_requests[3].clone().into()]);
}

#[test]
fn querying_unbond_requests_details() {
    let mut deps = mock_dependencies();
    let state = State::default();

    let unbond_requests = vec![
        UnbondRequest {
            id: 1,
            user: Addr::unchecked("alice"),
            shares: Uint128::new(123),
        },
        UnbondRequest {
            id: 1,
            user: Addr::unchecked("bob"),
            shares: Uint128::new(234),
        },
        UnbondRequest {
            id: 1,
            user: Addr::unchecked("charlie"),
            shares: Uint128::new(345),
        },
        UnbondRequest {
            id: 2,
            user: Addr::unchecked("alice"),
            shares: Uint128::new(456),
        },
        UnbondRequest {
            id: 3,
            user: Addr::unchecked("alice"),
            shares: Uint128::new(555),
        },
    ];

    let pending = PendingBatch {
        id: 3,
        ustake_to_burn: Uint128::new(1000),
        est_unbond_start_time: 20000,
    };

    state.pending_batch.save(deps.as_mut().storage, &pending).unwrap();

    let batches = vec![
        Batch {
            id: 1,
            reconciled: false,
            total_shares: Uint128::new(123),
            uluna_unclaimed: Uint128::new(678),
            est_unbond_end_time: 10000,
        },
        Batch {
            id: 2,
            reconciled: false,
            total_shares: Uint128::new(234),
            uluna_unclaimed: Uint128::new(789),
            est_unbond_end_time: 15000,
        },
    ];

    for batch in &batches {
        state.previous_batches.save(deps.as_mut().storage, batch.id, batch).unwrap();
    }

    for unbond_request in &unbond_requests {
        state
            .unbond_requests
            .save(
                deps.as_mut().storage,
                (unbond_request.id, &Addr::unchecked(unbond_request.user.clone())),
                unbond_request,
            )
            .unwrap();
    }

    let res: Vec<UnbondRequestsByUserResponseItemDetails> = query_helper_env(
        deps.as_ref(),
        QueryMsg::UnbondRequestsByUserDetails {
            user: "alice".to_string(),
            start_after: None,
            limit: None,
        },
        12000,
    );
    assert_eq!(
        res,
        vec![
            UnbondRequestsByUserResponseItemDetails {
                id: 1,
                shares: Uint128::new(123),
                state: "COMPLETED".to_string(),
                batch: Some(batches[0].clone()),
                pending: None
            },
            UnbondRequestsByUserResponseItemDetails {
                id: 2,
                shares: Uint128::new(456),
                state: "UNBONDING".to_string(),
                batch: Some(batches[1].clone()),
                pending: None
            },
            UnbondRequestsByUserResponseItemDetails {
                id: 3,
                shares: Uint128::new(555),
                state: "PENDING".to_string(),
                batch: None,
                pending: Some(pending)
            }
        ]
    );
}

//--------------------------------------------------------------------------------------------------
// Delegations
//--------------------------------------------------------------------------------------------------

#[test]
fn computing_undelegations() -> StdResult<()> {
    let deps = mock_dependencies();
    let state = State::default();
    let current_delegations = vec![
        Delegation::new("alice", 400),
        Delegation::new("bob", 300),
        Delegation::new("charlie", 200),
    ];
    // Target: (400 + 300 + 200 - 451) / 3 = 149
    // Remainder: 2
    // Alice:   400 - (149 + 2) = 249
    // Bob:     300 - (149 + 0) = 151
    // Charlie: 200 - (149 + 0) = 51
    let new_undelegations = compute_undelegations(
        &state,
        deps.as_ref().storage,
        Uint128::new(451),
        &current_delegations,
        current_delegations.iter().map(|a| a.validator.to_string()).collect_vec(),
    )?;
    let expected = vec![
        Undelegation::new("alice", 249),
        Undelegation::new("bob", 151),
        Undelegation::new("charlie", 51),
    ];
    assert_eq!(new_undelegations, expected);
    Ok(())
}
#[test]
fn computing_redelegations_for_removal() -> StdResult<()> {
    let deps = mock_dependencies();
    let state = State::default();
    let current_delegations = vec![
        Delegation::new("alice", 13000),
        Delegation::new("bob", 12000),
        Delegation::new("charlie", 11000),
        Delegation::new("dave", 10000),
    ];
    // Suppose Dave will be removed
    // uluna_per_validator = (13000 + 12000 + 11000 + 10000) / 3 = 15333
    // remainder = 1
    // to Alice:   15333 + 1 - 13000 = 2334
    // to Bob:     15333 + 0 - 12000 = 3333
    // to Charlie: 15333 + 0 - 11000 = 4333
    let expected = vec![
        Redelegation::new("dave", "alice", 2334),
        Redelegation::new("dave", "bob", 3333),
        Redelegation::new("dave", "charlie", 4333),
    ];
    assert_eq!(
        compute_redelegations_for_removal(
            &state,
            deps.as_ref().storage,
            &current_delegations[3],
            &current_delegations[..3],
            current_delegations[..3].iter().map(|a| a.validator.to_string()).collect_vec()
        )?,
        expected,
    );
    Ok(())
}

#[test]
fn computing_redelegations_for_rebalancing() -> StdResult<()> {
    let deps = mock_dependencies();
    let state = State::default();
    let current_delegations = vec![
        Delegation::new("alice", 69420),
        Delegation::new("bob", 1234),
        Delegation::new("charlie", 88888),
        Delegation::new("dave", 40471),
        Delegation::new("evan", 2345),
    ];
    // uluna_per_validator = (69420 + 88888 + 1234 + 40471 + 2345) / 4 = 40471
    // remainer = 3
    // src_delegations:
    //  - alice:   69420 - (40471 + 3) = 28946
    //  - charlie: 88888 - (40471 + 0) = 48417
    // dst_delegations:
    //  - bob:     (40471 + 0) - 1234  = 39237
    //  - evan:    (40471 + 0) - 2345  = 38126
    //
    // Round 1: alice --(28946)--> bob
    // src_delegations:
    //  - charlie: 48417
    // dst_delegations:
    //  - bob:     39237 - 28946 = 10291
    //  - evan:    38126
    //
    // Round 2: charlie --(10291)--> bob
    // src_delegations:
    //  - charlie: 48417 - 10291 = 38126
    // dst_delegations:
    //  - evan:    38126
    //
    // Round 3: charlie --(38126)--> evan
    // Queues are emptied
    let expected = vec![
        Redelegation::new("alice", "bob", 28946),
        Redelegation::new("charlie", "bob", 10291),
        Redelegation::new("charlie", "evan", 38126),
    ];
    assert_eq!(
        compute_redelegations_for_rebalancing(
            &state,
            deps.as_ref().storage,
            &current_delegations,
            current_delegations.iter().map(|a| a.validator.to_string()).collect_vec()
        )?,
        expected,
    );
    Ok(())
}

#[test]
fn computing_redelegations_for_rebalancing_complex() -> StdResult<()> {
    let mut deps = mock_dependencies();
    let state = State::default();
    state.delegation_goal.save(
        deps.as_mut().storage,
        &eris::hub::WantedDelegationsShare {
            tune_time: 0,
            tune_period: 0,
            shares: vec![
                ("charlie".to_string(), Decimal::from_str("0.5")?),
                ("alice".to_string(), Decimal::from_str("0.25")?),
                ("bob".to_string(), Decimal::from_str("0.25")?),
            ],
        },
    )?;
    // ratio is good
    let current_delegations = vec![
        Delegation::new("alice", 50000),
        Delegation::new("bob", 50000),
        Delegation::new("charlie", 100000),
    ];
    assert_eq!(
        compute_redelegations_for_rebalancing(
            &state,
            deps.as_ref().storage,
            &current_delegations,
            current_delegations.iter().map(|a| a.validator.to_string()).collect_vec()
        )?,
        vec![],
    );
    // ratio is bad
    let current_delegations = vec![
        Delegation::new("unlisted", 25000),
        Delegation::new("alice", 25000),
        Delegation::new("bob", 50000),
        Delegation::new("charlie", 100000),
    ];
    assert_eq!(
        compute_redelegations_for_rebalancing(
            &state,
            deps.as_ref().storage,
            &current_delegations,
            current_delegations.iter().map(|a| a.validator.to_string()).collect_vec()
        )?,
        vec![Redelegation::new("unlisted", "alice", 25000)],
    );
    // ratio is bad
    let current_delegations = vec![
        Delegation::new("charlie", 100000),
        Delegation::new("unlisted", 50000),
        Delegation::new("alice", 25000),
        Delegation::new("bob", 25000),
    ];
    assert_eq!(
        compute_redelegations_for_rebalancing(
            &state,
            deps.as_ref().storage,
            &current_delegations,
            current_delegations.iter().map(|a| a.validator.to_string()).collect_vec()
        )?,
        vec![
            Redelegation::new("unlisted", "alice", 25000),
            Redelegation::new("unlisted", "bob", 25000)
        ],
    );
    // ratio is bad
    let current_delegations = vec![
        Delegation::new("charlie", 150002),
        Delegation::new("alice", 20000),
        Delegation::new("bob", 20000),
    ];
    assert_eq!(
        compute_redelegations_for_rebalancing(
            &state,
            deps.as_ref().storage,
            &current_delegations,
            current_delegations.iter().map(|a| a.validator.to_string()).collect_vec()
        )?,
        vec![
            Redelegation::new("charlie", "alice", 27500),
            Redelegation::new("charlie", "bob", 27500)
        ],
    );
    Ok(())
}

//--------------------------------------------------------------------------------------------------
// Coins
//--------------------------------------------------------------------------------------------------

#[test]
fn adding_coins() {
    let mut coins = Coins(vec![]);

    coins.add(&Coin::new(12345, "uatom")).unwrap();
    assert_eq!(coins.0, vec![Coin::new(12345, "uatom")]);

    coins.add(&Coin::new(23456, CONTRACT_DENOM)).unwrap();
    assert_eq!(coins.0, vec![Coin::new(12345, "uatom"), Coin::new(23456, CONTRACT_DENOM)]);

    coins.add_many(&Coins(vec![Coin::new(76543, "uatom"), Coin::new(69420, "uusd")])).unwrap();
    assert_eq!(
        coins.0,
        vec![Coin::new(88888, "uatom"), Coin::new(23456, CONTRACT_DENOM), Coin::new(69420, "uusd")]
    );
}

#[test]
fn receiving_funds() {
    let err = parse_received_fund(&[], CONTRACT_DENOM).unwrap_err();
    assert_eq!(err, StdError::generic_err("must deposit exactly one coin; received 0"));

    let err = parse_received_fund(
        &[Coin::new(12345, "uatom"), Coin::new(23456, CONTRACT_DENOM)],
        CONTRACT_DENOM,
    )
    .unwrap_err();
    assert_eq!(err, StdError::generic_err("must deposit exactly one coin; received 2"));

    let err = parse_received_fund(&[Coin::new(12345, "uatom")], CONTRACT_DENOM).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err(format!("expected {} deposit, received uatom", CONTRACT_DENOM))
    );

    let err = parse_received_fund(&[Coin::new(0, CONTRACT_DENOM)], CONTRACT_DENOM).unwrap_err();
    assert_eq!(err, StdError::generic_err("deposit amount must be non-zero"));

    let amount = parse_received_fund(&[Coin::new(69420, CONTRACT_DENOM)], CONTRACT_DENOM).unwrap();
    assert_eq!(amount, Uint128::new(69420));
}

#[test]
fn running_dedup() {
    let mut validators = vec![
        "terraveloper1".to_string(),
        "terraveloper2".to_string(),
        "terraveloper3".to_string(),
        "terraveloper1".to_string(),
        "terraveloper3".to_string(),
        "terraveloper3".to_string(),
        "terraveloper2".to_string(),
        "terraveloper1".to_string(),
        "terraveloper1".to_string(),
    ];
    dedupe(&mut validators);

    assert_eq!(
        validators,
        vec!["terraveloper1".to_string(), "terraveloper2".to_string(), "terraveloper3".to_string()]
    )
}

pub fn check_received_coin(amount: u128) -> SubMsg {
    SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: MOCK_CONTRACT_ADDR.to_string(),
        msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::CheckReceivedCoin {
            snapshot: coin(amount, CONTRACT_DENOM),
        }))
        .unwrap(),
        funds: vec![],
    }))
}
