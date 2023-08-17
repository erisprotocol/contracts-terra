use cosmwasm_std::{attr, StdResult, Uint128};
use eris_tests::gov_helper::EscrowHelper;
use eris_tests::{mock_app, AppExtension, EventChecker};
use std::vec;

use eris::emp_gauges::{
    ConfigResponse, EmpInfo, ExecuteMsg, GaugeInfoResponse, VotedValidatorInfoResponse,
};

#[test]
fn update_configs() {
    let mut router = mock_app();
    let helper = EscrowHelper::init(&mut router, false);

    let config = helper.emp_query_config(&mut router).unwrap();
    assert_eq!(config.validators_limit, 30);

    let result = helper
        .emp_execute_sender(
            &mut router,
            ExecuteMsg::UpdateConfig {
                validators_limit: Some(40),
            },
            "user",
        )
        .unwrap_err();

    assert_eq!("Generic error: unauthorized", result.root_cause().to_string());

    helper
        .emp_execute(
            &mut router,
            ExecuteMsg::UpdateConfig {
                validators_limit: Some(40),
            },
        )
        .unwrap();

    let config = helper.emp_query_config(&mut router).unwrap();
    assert_eq!(config.validators_limit, 40);
}

#[test]
fn add_points() -> StdResult<()> {
    let mut router = mock_app();
    let helper = EscrowHelper::init(&mut router, false);

    let result = helper
        .emp_execute(
            &mut router,
            ExecuteMsg::AddEmps {
                emps: vec![(
                    "unknown-validator".to_string(),
                    vec![EmpInfo {
                        decaying_period: Some(3),
                        umerit_points: Uint128::new(1000000),
                    }],
                )],
            },
        )
        .unwrap_err();

    assert_eq!("Invalid validator address: unknown-validator", result.root_cause().to_string());

    let result = helper
        .emp_execute(
            &mut router,
            ExecuteMsg::AddEmps {
                emps: vec![
                    (
                        "val1".to_string(),
                        vec![
                            EmpInfo {
                                decaying_period: Some(2 * 4), // 2 months
                                umerit_points: Uint128::new(2000000),
                            },
                            EmpInfo {
                                decaying_period: None,
                                umerit_points: Uint128::new(1000000),
                            },
                        ],
                    ),
                    (
                        "val2".to_string(),
                        vec![
                            EmpInfo {
                                decaying_period: Some(2 * 4), // 2 months
                                umerit_points: Uint128::new(1000000),
                            },
                            EmpInfo {
                                decaying_period: None,
                                umerit_points: Uint128::new(2000000),
                            },
                        ],
                    ),
                ],
            },
        )
        .unwrap();

    result.assert_attribute("wasm", attr("emps", "val1=3000000")).unwrap();
    result.assert_attribute("wasm", attr("emps", "val2=3000000")).unwrap();

    let old_period = router.block_period();
    router.next_period(4);
    let current_period = router.block_period();
    assert_eq!(old_period + 4, current_period);

    let result = helper.emp_execute(&mut router, ExecuteMsg::TuneEmps {}).unwrap();
    result.assert_attribute("wasm", attr("emps", "val1=2000000")).unwrap();
    result.assert_attribute("wasm", attr("emps", "val2=2500000")).unwrap();

    let result = helper
        .emp_execute(
            &mut router,
            ExecuteMsg::AddEmps {
                emps: vec![
                    (
                        "val3".to_string(),
                        vec![EmpInfo {
                            decaying_period: Some(4), // 1 months
                            umerit_points: Uint128::new(1000000),
                        }],
                    ),
                    (
                        "val2".to_string(),
                        vec![EmpInfo {
                            decaying_period: Some(4), // 1 months
                            umerit_points: Uint128::new(1000000),
                        }],
                    ),
                    (
                        "val4".to_string(),
                        vec![EmpInfo {
                            decaying_period: None,
                            umerit_points: Uint128::new(500000),
                        }],
                    ),
                ],
            },
        )
        .unwrap();
    result.assert_attribute("wasm", attr("emps", "val1=2000000")).unwrap();
    result.assert_attribute("wasm", attr("emps", "val2=3500000")).unwrap();
    result.assert_attribute("wasm", attr("emps", "val3=1000000")).unwrap();
    result.assert_attribute("wasm", attr("emps", "val4=500000")).unwrap();

    let result = helper.emp_query_tune_info(&mut router).unwrap();
    assert_eq!(
        result,
        GaugeInfoResponse {
            tune_ts: 1669593600,
            tune_period: 4,
            emp_points: vec![
                ("val2".to_string(), Uint128::new(3500000)),
                ("val1".to_string(), Uint128::new(2000000)),
                ("val3".to_string(), Uint128::new(1000000)),
                ("val4".to_string(), Uint128::new(500000)),
            ]
        }
    );

    router.next_period(2);
    let result = helper.emp_execute(&mut router, ExecuteMsg::TuneEmps {}).unwrap();
    result.assert_attribute("wasm", attr("emps", "val1=1500000")).unwrap();
    result.assert_attribute("wasm", attr("emps", "val2=2750000")).unwrap();
    result.assert_attribute("wasm", attr("emps", "val3=500000")).unwrap();
    result.assert_attribute("wasm", attr("emps", "val4=500000")).unwrap();

    let result = helper.emp_execute(&mut router, ExecuteMsg::TuneEmps {}).unwrap();
    result.assert_attribute("wasm", attr("emps", "val1=1500000")).unwrap();
    result.assert_attribute("wasm", attr("emps", "val2=2750000")).unwrap();
    result.assert_attribute("wasm", attr("emps", "val3=500000")).unwrap();
    result.assert_attribute("wasm", attr("emps", "val4=500000")).unwrap();

    router.next_period(2);
    let result = helper.emp_execute(&mut router, ExecuteMsg::TuneEmps {}).unwrap();
    result.assert_attribute("wasm", attr("emps", "val1=1000000")).unwrap();
    result.assert_attribute("wasm", attr("emps", "val2=2000000")).unwrap();
    result.assert_attribute("wasm", attr("emps", "val4=500000")).unwrap();

    router.next_period(4);
    let result = helper.emp_execute(&mut router, ExecuteMsg::TuneEmps {}).unwrap();
    result.assert_attribute("wasm", attr("emps", "val1=1000000")).unwrap();
    result.assert_attribute("wasm", attr("emps", "val2=2000000")).unwrap();
    result.assert_attribute("wasm", attr("emps", "val4=500000")).unwrap();

    let result = helper.emp_execute(&mut router, ExecuteMsg::TuneEmps {}).unwrap();
    result.assert_attribute("wasm", attr("emps", "val1=1000000")).unwrap();
    result.assert_attribute("wasm", attr("emps", "val2=2000000")).unwrap();
    result.assert_attribute("wasm", attr("emps", "val4=500000")).unwrap();

    let result = helper.emp_query_validator_history(&mut router, "val1", 0).unwrap();
    assert_eq!(
        result,
        VotedValidatorInfoResponse {
            voting_power: Uint128::new(2000000),
            fixed_amount: Uint128::new(1000000),
            slope: Uint128::new(250000)
        }
    );

    let result = helper.emp_query_validator_history(&mut router, "val1", 1).unwrap();
    assert_eq!(
        result,
        VotedValidatorInfoResponse {
            voting_power: Uint128::new(1750000),
            fixed_amount: Uint128::new(1000000),
            slope: Uint128::new(250000)
        }
    );

    let result = helper.emp_query_validator_history(&mut router, "val4", 0).unwrap();
    assert_eq!(
        result,
        VotedValidatorInfoResponse {
            voting_power: Uint128::zero(),
            fixed_amount: Uint128::zero(),
            slope: Uint128::zero()
        }
    );

    let result = helper.emp_query_validator_history(&mut router, "val4", 2).unwrap();
    assert_eq!(
        result,
        VotedValidatorInfoResponse {
            voting_power: Uint128::zero(),
            fixed_amount: Uint128::zero(),
            slope: Uint128::zero()
        }
    );

    let result = helper.emp_query_validator_history(&mut router, "val4", 4).unwrap();
    assert_eq!(
        result,
        VotedValidatorInfoResponse {
            voting_power: Uint128::zero(),
            fixed_amount: Uint128::new(500000),
            slope: Uint128::zero()
        }
    );

    let result = helper.emp_query_validator_history(&mut router, "val4", 5).unwrap();
    assert_eq!(
        result,
        VotedValidatorInfoResponse {
            voting_power: Uint128::zero(),
            fixed_amount: Uint128::new(500000),
            slope: Uint128::zero()
        }
    );

    Ok(())
}

#[test]
fn check_kick_holders_works() -> StdResult<()> {
    let mut router = mock_app();
    let helper = EscrowHelper::init(&mut router, false);

    let result = helper
        .emp_execute(
            &mut router,
            ExecuteMsg::AddEmps {
                emps: vec![
                    (
                        "val1".to_string(),
                        vec![
                            EmpInfo {
                                decaying_period: Some(2 * 4), // 2 months
                                umerit_points: Uint128::new(2000000),
                            },
                            EmpInfo {
                                decaying_period: None,
                                umerit_points: Uint128::new(1000000),
                            },
                        ],
                    ),
                    (
                        "val2".to_string(),
                        vec![
                            EmpInfo {
                                decaying_period: Some(2 * 4), // 2 months
                                umerit_points: Uint128::new(1000000),
                            },
                            EmpInfo {
                                decaying_period: None,
                                umerit_points: Uint128::new(2000000),
                            },
                        ],
                    ),
                ],
            },
        )
        .unwrap();

    result.assert_attribute("wasm", attr("emps", "val1=3000000")).unwrap();
    result.assert_attribute("wasm", attr("emps", "val2=3000000")).unwrap();

    helper.hub_remove_validator(&mut router, "val2").unwrap();

    let result = helper.emp_execute(&mut router, ExecuteMsg::TuneEmps {}).unwrap();
    result.assert_attribute("wasm", attr("emps", "val1=3000000")).unwrap();
    let err = result.assert_attribute("wasm", attr("emps", "val2=3000000")).unwrap_err();
    assert_eq!(err.to_string(), "Could not find key: emps value: val2=3000000");

    Ok(())
}

#[test]
fn add_points_later() -> StdResult<()> {
    let mut router = mock_app();
    let helper = EscrowHelper::init(&mut router, false);

    let result = helper
        .emp_execute(
            &mut router,
            ExecuteMsg::AddEmps {
                emps: vec![(
                    "val2".to_string(),
                    vec![EmpInfo {
                        decaying_period: None,
                        umerit_points: Uint128::new(1000000),
                    }],
                )],
            },
        )
        .unwrap();
    result.assert_attribute("wasm", attr("emps", "val2=1000000")).unwrap();

    router.next_period(4);

    let result = helper
        .emp_execute(
            &mut router,
            ExecuteMsg::AddEmps {
                emps: vec![(
                    "val2".to_string(),
                    vec![EmpInfo {
                        decaying_period: None,
                        umerit_points: Uint128::new(2000000),
                    }],
                )],
            },
        )
        .unwrap();
    result.assert_attribute("wasm", attr("emps", "val2=3000000")).unwrap();

    let result = helper
        .emp_execute(
            &mut router,
            ExecuteMsg::AddEmps {
                emps: vec![(
                    "val2".to_string(),
                    vec![EmpInfo {
                        decaying_period: None,
                        umerit_points: Uint128::new(2000000),
                    }],
                )],
            },
        )
        .unwrap();
    result.assert_attribute("wasm", attr("emps", "val2=5000000")).unwrap();

    Ok(())
}

#[test]
fn check_update_owner() {
    let mut router = mock_app();
    let helper = EscrowHelper::init(&mut router, false);

    let new_owner = String::from("new_owner");

    // New owner
    let msg = ExecuteMsg::ProposeNewOwner {
        new_owner: new_owner.clone(),
        expires_in: 100, // seconds
    };

    // Unauthed check
    let err = helper.emp_execute_sender(&mut router, msg.clone(), "not_owner").unwrap_err();

    assert_eq!(err.root_cause().to_string(), "Generic error: Unauthorized");

    // Claim before proposal
    let err = helper
        .emp_execute_sender(&mut router, ExecuteMsg::ClaimOwnership {}, new_owner.clone())
        .unwrap_err();
    assert_eq!(err.root_cause().to_string(), "Generic error: Ownership proposal not found");

    // Propose new owner
    helper.emp_execute_sender(&mut router, msg, "owner").unwrap();

    // Claim from invalid addr
    let err = helper
        .emp_execute_sender(&mut router, ExecuteMsg::ClaimOwnership {}, "invalid_addr")
        .unwrap_err();

    assert_eq!(err.root_cause().to_string(), "Generic error: Unauthorized");

    // Claim ownership
    helper
        .emp_execute_sender(&mut router, ExecuteMsg::ClaimOwnership {}, new_owner.clone())
        .unwrap();

    // Let's query the contract state
    let res: ConfigResponse = helper.emp_query_config(&mut router).unwrap();

    assert_eq!(res.owner, new_owner)
}
