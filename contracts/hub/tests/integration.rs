use anyhow::{Ok, Result};
use cosmwasm_std::{attr, coin, coins, Addr, Decimal, Delegation, Event, FullDelegation, Uint128};
use cw_multi_test::App;
use eris::hub::{DelegationStrategy, StateResponse, WantedDelegationsResponse};
use eris_tests::{gov_helper::EscrowHelper, TerraAppExtension};
use eris_tests::{mock_app, mock_app_validators, EventChecker};
use itertools::Itertools;
use std::str::FromStr;
use std::vec;

use eris::{
    emp_gauges::EmpInfo,
    governance_helper::WEEK,
    hub::{ConfigResponse, ExecuteMsg, FeeConfig},
};

#[test]
fn update_configs() -> Result<()> {
    let mut router = mock_app();
    let helper = EscrowHelper::init(&mut router, false);

    let config = helper.hub_query_config(&mut router)?;
    assert_eq!(
        config,
        ConfigResponse {
            owner: "owner".to_string(),
            new_owner: None,
            stake_token: helper.base.amp_token.get_address_string(),
            epoch_period: 259200,
            unbond_period: 1814400,
            validators: vec![
                "val1".to_string(),
                "val2".to_string(),
                "val3".to_string(),
                "val4".to_string()
            ],
            fee_config: FeeConfig {
                protocol_fee_contract: Addr::unchecked("fee"),
                protocol_reward_fee: Decimal::from_ratio(1u128, 100u128)
            },
            delegation_strategy: eris::hub::DelegationStrategy::Gauges {
                amp_gauges: helper.base.amp_gauges.get_address_string(),
                emp_gauges: Some(helper.base.emp_gauges.get_address_string()),
                amp_factor_bps: 5000,
                min_delegation_bps: 100,
                max_delegation_bps: 2500,
                validator_count: 5,
            },
        }
    );

    let result = helper
        .hub_execute_sender(
            &mut router,
            ExecuteMsg::UpdateConfig {
                protocol_reward_fee: Some(Decimal::from_str("0")?),
                delegation_strategy: None,
                protocol_fee_contract: None,
            },
            Addr::unchecked("user"),
        )
        .unwrap_err();

    assert_eq!("Generic error: unauthorized: sender is not owner", result.root_cause().to_string());

    helper
        .hub_execute(
            &mut router,
            ExecuteMsg::UpdateConfig {
                protocol_reward_fee: Some(Decimal::from_str("0.1")?),
                protocol_fee_contract: Some("fee_new".to_string()),
                delegation_strategy: Some(eris::hub::DelegationStrategy::Uniform),
            },
        )
        .unwrap();

    let config = helper.hub_query_config(&mut router)?;
    assert_eq!(
        config,
        ConfigResponse {
            owner: "owner".to_string(),
            new_owner: None,
            stake_token: helper.base.amp_token.get_address_string(),
            epoch_period: 259200,
            unbond_period: 1814400,
            validators: vec![
                "val1".to_string(),
                "val2".to_string(),
                "val3".to_string(),
                "val4".to_string()
            ],
            fee_config: FeeConfig {
                protocol_fee_contract: Addr::unchecked("fee_new"),
                protocol_reward_fee: Decimal::from_ratio(10u128, 100u128)
            },
            delegation_strategy: eris::hub::DelegationStrategy::Uniform
        }
    );

    Ok(())
}

#[test]
fn happy_case() -> Result<()> {
    let mut router = mock_app_validators(Some(100));
    let router_ref = &mut router;
    let helper = EscrowHelper::init(router_ref, false);

    for i in 5..100 {
        helper.hub_add_validator(router_ref, format!("val{0}", i))?;
    }

    helper.hub_bond(router_ref, "user1", 100_000000, "uluna")?;

    helper.ve_lock_lp(router_ref, "user1", 1_000000, WEEK * 3)?;
    helper.ve_lock_lp(router_ref, "user2", 1_000000, WEEK * 104)?;
    helper.ve_lock_lp(router_ref, "user3", 1_000000, WEEK * 3)?;
    helper.ve_lock_lp(router_ref, "user4", 10_000000, WEEK * 50)?;
    helper.ve_lock_lp(router_ref, "user5", 10_000000, WEEK * 50)?;

    helper.amp_vote(router_ref, "user1", vec![("val1".into(), 5000), ("val4".into(), 5000)])?;
    helper.amp_vote(router_ref, "user2", vec![("val1".into(), 9000), ("val2".into(), 1000)])?;
    helper.amp_vote(router_ref, "user3", vec![("val1".into(), 1000), ("val4".into(), 9000)])?;
    helper.amp_vote(router_ref, "user4", vec![("val5".into(), 2400), ("val6".into(), 7600)])?;
    helper.amp_vote(router_ref, "user5", vec![("val7".into(), 2500), ("val8".into(), 7500)])?;

    router_ref.next_period(1);

    helper.emp_add_points(
        router_ref,
        vec![
            (
                "val1".into(),
                vec![
                    EmpInfo {
                        umerit_points: Uint128::new(10_000000),
                        decaying_period: None,
                    },
                    EmpInfo {
                        umerit_points: Uint128::new(1_000000),
                        decaying_period: Some(12), // 3 months
                    },
                ],
            ),
            point("val2", 20_000000, Some(12)),
            point("val9", 1_000000, Some(3)),
            point("val10", 20_000000, Some(3)),
        ],
    )?;

    // admin at each first monday of month
    helper.amp_tune(router_ref)?;
    helper.hub_tune(router_ref)?;

    let wanted = helper.hub_query_wanted_delegations(router_ref)?;

    // val5, val7, val9 missing due to too few spaces (only 5 available).
    assert_eq!(
        wanted.delegations,
        vec![
            ("val2".into(), Uint128::new(22500152)),
            ("val10".into(), Uint128::new(22019299)),
            ("val6".into(), Uint128::new(19467469)),
            ("val8".into(), Uint128::new(19211318)),
            ("val1".into(), Uint128::new(16801760))
        ]
    );

    let results = helper.hub_rebalance(router_ref)?;
    results.assert_attribute("wasm-erishub/rebalanced", attr("uluna_moved", "83198238")).unwrap();

    let delegations = helper.hub_query_all_delegations(router_ref)?;

    let sum: u128 = delegations.iter().map(|d| d.amount.amount.u128()).sum();
    assert_eq!(sum, 100_000000u128);

    assert_eq!(
        delegations
            .into_iter()
            .map(|d| (d.validator, d.amount.amount))
            .sorted_by(|(_, a), (_, b)| b.cmp(a))
            .collect::<Vec<_>>(),
        vec![
            ("val2".into(), Uint128::new(22500152)),
            ("val10".into(), Uint128::new(22019299)),
            ("val6".into(), Uint128::new(19467469)),
            ("val8".into(), Uint128::new(19211318)),
            ("val1".into(), Uint128::new(16801760 + 2))
        ]
    );

    Ok(())
}

#[test]
fn config_does_not_change_exchange_rate() -> Result<()> {
    let mut router = mock_app_validators(Some(100));
    let router_ref = &mut router;
    let helper = EscrowHelper::init(router_ref, true);

    router_ref.next_block(60 * 60 * 24);

    let result = helper.hub_query_delegation(router_ref, "val1")?;
    assert_eq!(result, None);

    let (ustake_minted, new_luna) = bond_and_harvest(&helper, router_ref)?;

    // check that the state is correct based on harvest
    let expected_response = StateResponse {
        exchange_rate: Decimal::from_ratio(new_luna, ustake_minted),
        total_ustake: ustake_minted,
        total_uluna: new_luna,
        unlocked_coins: vec![],
        unbonding: Uint128::zero(),
        available: Uint128::zero(),
        tvl_uluna: new_luna,
    };
    let state = helper.hub_query_state(router_ref)?;
    assert_eq!(state, expected_response);

    let wanted = helper.hub_query_wanted_delegations(router_ref)?;
    assert_eq!(
        wanted,
        WantedDelegationsResponse {
            tune_time_period: None,
            delegations: vec![
                ("val1".to_string(), Uint128::new(447940458)),
                ("val2".to_string(), Uint128::new(447940458)),
                ("val3".to_string(), Uint128::new(447940458)),
                ("val4".to_string(), Uint128::new(447940458)),
            ]
        }
    );

    // change to gauges principle
    // this only stores the gauges, but does not yet change delegations
    helper.hub_execute(
        router_ref,
        ExecuteMsg::UpdateConfig {
            protocol_fee_contract: None,
            protocol_reward_fee: None,
            delegation_strategy: Some(DelegationStrategy::Gauges {
                amp_gauges: helper.base.amp_gauges.get_address_string(),
                emp_gauges: None, //Some(helper.base.emp_gauges.get_address_string()),
                amp_factor_bps: 5000,
                min_delegation_bps: 100,
                max_delegation_bps: 10000,
                validator_count: 5,
            }),
        },
    )?;

    // after config change exchange_rate needs to be the same
    let state = helper.hub_query_state(router_ref)?;
    assert_eq!(state, expected_response);

    // try tune without vAMP
    helper.hub_tune(router_ref).expect_err("Generic error: No vAMP. Vote first before tuning.");

    // add vAMP for user1 and tune it
    setup_vamp_user1(&helper, router_ref)?;

    // tune with vAMP + expect same echange rate
    helper.hub_tune(router_ref)?;
    let state = helper.hub_query_state(router_ref)?;
    assert_eq!(state, expected_response);

    let wanted = helper.hub_query_wanted_delegations(router_ref)?;
    assert_eq!(
        wanted,
        WantedDelegationsResponse {
            tune_time_period: Some((1667952000, 1)),
            delegations: vec![
                ("val1".to_string(), Uint128::new(1254_233285)),
                ("val2".to_string(), Uint128::new(537_528549)),
            ]
        }
    );

    let delegations = helper.hub_query_all_delegations(router_ref)?;
    assert_eq!(
        delegations,
        vec![
            // first bond
            delegation(&helper, "val1", 100_000000),
            // second bond
            delegation(&helper, "val2", 200_000000),
            // harvest
            delegation(&helper, "val3", 1491_761835)
        ]
    );

    // rebalance stakes + expect same exchange rate
    helper.hub_rebalance(router_ref)?;
    let state = helper.hub_query_state(router_ref)?;
    assert_eq!(state, expected_response);

    let wanted = helper.hub_query_wanted_delegations(router_ref)?;
    assert_eq!(
        wanted,
        WantedDelegationsResponse {
            tune_time_period: Some((1667952000, 1)),
            delegations: vec![
                ("val1".to_string(), Uint128::new(1254_233285)),
                ("val2".to_string(), Uint128::new(537_528549)),
            ]
        }
    );

    let delegations = helper.hub_query_all_delegations(router_ref)?;
    assert_eq!(
        delegations,
        vec![
            // 70 % (rounding from wanted added to highest val)
            delegation(&helper, "val1", 1254_233286),
            // 30 %
            delegation(&helper, "val2", 537_528549),
        ]
    );

    Ok(())
}

#[test]
fn config_does_not_change_exchange_rate_emps() -> Result<()> {
    let mut router = mock_app_validators(Some(100));
    let router_ref = &mut router;
    let helper = EscrowHelper::init(router_ref, true);

    router_ref.next_block(60 * 60 * 24);

    let result = helper.hub_query_delegation(router_ref, "val1")?;
    assert_eq!(result, None);

    let (ustake_minted, new_luna) = bond_and_harvest(&helper, router_ref)?;

    // check that the state is correct based on harvest
    let expected_response = StateResponse {
        exchange_rate: Decimal::from_ratio(new_luna, ustake_minted),
        total_ustake: ustake_minted,
        total_uluna: new_luna,
        unlocked_coins: vec![],
        unbonding: Uint128::zero(),
        available: Uint128::zero(),
        tvl_uluna: new_luna,
    };
    let state = helper.hub_query_state(router_ref)?;
    assert_eq!(state, expected_response);

    let wanted = helper.hub_query_wanted_delegations(router_ref)?;
    assert_eq!(
        wanted,
        WantedDelegationsResponse {
            tune_time_period: None,
            delegations: vec![
                ("val1".to_string(), Uint128::new(447940458)),
                ("val2".to_string(), Uint128::new(447940458)),
                ("val3".to_string(), Uint128::new(447940458)),
                ("val4".to_string(), Uint128::new(447940458)),
            ]
        }
    );

    // change to gauges principle
    // this only stores the gauges, but does not yet change delegations
    helper.hub_execute(
        router_ref,
        ExecuteMsg::UpdateConfig {
            protocol_fee_contract: None,
            protocol_reward_fee: None,
            delegation_strategy: Some(DelegationStrategy::Gauges {
                amp_gauges: helper.base.amp_gauges.get_address_string(),
                emp_gauges: Some(helper.base.emp_gauges.get_address_string()),
                amp_factor_bps: 5000,
                min_delegation_bps: 100,
                max_delegation_bps: 10000,
                validator_count: 5,
            }),
        },
    )?;

    // after config change exchange_rate needs to be the same
    let state = helper.hub_query_state(router_ref)?;
    assert_eq!(state, expected_response);

    // try tune without vAMP
    helper.hub_tune(router_ref).expect_err("Generic error: No vAMP. Vote first before tuning.");

    // add vAMP for user1 and tune it
    setup_vamp_user1(&helper, router_ref)?;

    helper.hub_tune(router_ref).expect_err("Generic error: EMP not tuned.");

    setup_emps(&helper, router_ref)?;

    // tune with vAMP + EMP + expect same echange rate
    helper.hub_tune(router_ref)?;
    let state = helper.hub_query_state(router_ref)?;
    assert_eq!(state, expected_response);

    let wanted = helper.hub_query_wanted_delegations(router_ref)?;
    assert_eq!(
        wanted,
        WantedDelegationsResponse {
            tune_time_period: Some((1667952000, 1)),
            delegations: vec![
                ("val1".to_string(), Uint128::new(627_116642)),
                ("val2".to_string(), Uint128::new(567_391247)),
                ("val3".to_string(), Uint128::new(298_626972)),
                ("val4".to_string(), Uint128::new(298_626972)),
            ]
        }
    );

    let delegations = helper.hub_query_all_delegations(router_ref)?;
    assert_eq!(
        delegations,
        vec![
            // first bond
            delegation(&helper, "val1", 100_000000),
            // second bond
            delegation(&helper, "val2", 200_000000),
            // harvest
            delegation(&helper, "val3", 1491_761835)
        ]
    );

    // rebalance stakes + expect same exchange rate
    helper.hub_rebalance(router_ref)?;
    let state = helper.hub_query_state(router_ref)?;
    assert_eq!(state, expected_response);

    let wanted = helper.hub_query_wanted_delegations(router_ref)?;
    assert_eq!(
        wanted,
        WantedDelegationsResponse {
            tune_time_period: Some((1667952000, 1)),
            delegations: vec![
                ("val1".to_string(), Uint128::new(627_116642)),
                ("val2".to_string(), Uint128::new(567_391247)),
                ("val3".to_string(), Uint128::new(298_626972)),
                ("val4".to_string(), Uint128::new(298_626972)),
            ]
        }
    );

    let delegations = helper.hub_query_all_delegations(router_ref)?;
    assert_eq!(
        delegations,
        vec![
            // 70% * 50% (vAMP) (rounding from wanted added to highest val)
            delegation(&helper, "val1", 627_116644),
            // 30% * 50% (vAMP) + 33,33% * 50% (EMP)
            delegation(&helper, "val2", 567_391247),
            // 33,33% * 50% (EMP)
            delegation(&helper, "val3", 298_626972),
            // 33,33% * 50% (EMP)
            delegation(&helper, "val4", 298_626972),
        ]
    );

    Ok(())
}

fn setup_emps(helper: &EscrowHelper, router_ref: &mut App) -> Result<(), anyhow::Error> {
    helper.emp_add_points(
        router_ref,
        vec![
            point("val2", 1_000000, None),
            point("val3", 1_000000, None),
            point("val4", 1_000000, None),
        ],
    )?;
    helper.emp_tune(router_ref)?;
    Ok(())
}

fn setup_vamp_user1(helper: &EscrowHelper, router_ref: &mut App) -> Result<(), anyhow::Error> {
    helper.ve_lock_lp(router_ref, "user1", 100_000000, WEEK * 3)?;
    helper.amp_vote(
        router_ref,
        "user1",
        vec![("val1".to_string(), 7000), ("val2".to_string(), 3000)],
    )?;
    router_ref.next_period(1);
    helper.amp_tune(router_ref)?;
    Ok(())
}

fn bond_and_harvest(
    helper: &EscrowHelper,
    router_ref: &mut cw_multi_test::App,
) -> Result<(Uint128, Uint128), anyhow::Error> {
    helper.hub_bond(router_ref, "user1", 100_000000, "uluna")?;
    helper.hub_bond(router_ref, "user2", 200_000000, "uluna")?;
    let state = helper.hub_query_state(router_ref)?;
    assert_eq!(
        state,
        StateResponse {
            exchange_rate: Decimal::one(),
            total_ustake: Uint128::new(300_000000),
            total_uluna: Uint128::new(300_000000),
            unlocked_coins: vec![],
            unbonding: Uint128::zero(),
            available: Uint128::zero(),
            tvl_uluna: Uint128::new(300_000000),
        }
    );
    let result = helper.hub_query_delegation(router_ref, "val1")?;
    assert_eq!(
        result,
        Some(FullDelegation {
            delegator: helper.base.hub.get_address(),
            validator: "val1".to_string(),
            amount: coin(100_000000, "uluna"),
            can_redelegate: coin(100_000000, "uluna"),
            accumulated_rewards: coins(502_250684, "uluna"),
        })
    );
    router_ref.next_block(60 * 60 * 24);
    let result = helper.hub_query_delegation(router_ref, "val1")?;
    assert_eq!(
        result,
        Some(FullDelegation {
            delegator: helper.base.hub.get_address(),
            validator: "val1".to_string(),
            amount: coin(100_000000, "uluna"),
            can_redelegate: coin(100_000000, "uluna"),
            // not sure why accumulated rewards are so high
            // for the test it is only important that the full rewards are compounded
            accumulated_rewards: coins(502_276712, "uluna"),
        })
    );
    let result = helper.hub_query_delegation(router_ref, "val2")?;
    assert_eq!(
        result,
        Some(FullDelegation {
            delegator: helper.base.hub.get_address(),
            validator: "val2".to_string(),
            amount: coin(200_000000, "uluna"),
            can_redelegate: coin(200_000000, "uluna"),
            accumulated_rewards: coins(1004_553424, "uluna"),
        })
    );
    let result = helper.hub_harvest(router_ref)?;
    let amount = Uint128::new(502_276712).checked_mul(Uint128::new(3))?;
    let fee = amount * Decimal::percent(1);
    let bonded = amount.checked_sub(fee)?;
    result.assert_attribute("wasm", attr("action", "erishub/harvest"))?;
    result.assert_attribute("wasm", attr("action", "erishub/reinvest"))?;
    result.assert_event(
        &Event::new("wasm-erishub/harvested")
            .add_attribute("_contract_addr", "contract1")
            .add_attribute("uluna_bonded", bonded.to_string())
            .add_attribute("uluna_protocol_fee", fee.to_string()),
    );
    let ustake_minted = Uint128::new(300_000000);
    let new_luna = Uint128::new(300_000000).checked_add(bonded)?;
    Ok((ustake_minted, new_luna))
}

pub fn delegation(helper: &EscrowHelper, validator: impl Into<String>, amount: u128) -> Delegation {
    Delegation {
        delegator: helper.base.hub.get_address(),
        validator: validator.into(),
        amount: coin(amount, "uluna"),
    }
}

pub fn point(
    val: impl Into<String>,
    amount: u128,
    decaying: Option<u64>,
) -> (String, Vec<EmpInfo>) {
    (
        val.into(),
        vec![EmpInfo {
            umerit_points: Uint128::new(amount),
            decaying_period: decaying, // 3 months
        }],
    )
}
