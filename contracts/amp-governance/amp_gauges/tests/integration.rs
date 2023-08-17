use cosmwasm_std::{attr, StdResult, Uint128};
use eris::governance_helper::WEEK;
use eris_tests::gov_helper::EscrowHelper;
use eris_tests::{mock_app, AppExtension, EventChecker};
use std::vec;

use eris::amp_gauges::{ConfigResponse, ExecuteMsg};

#[test]
fn update_configs() -> StdResult<()> {
    let mut router = mock_app();
    let helper = EscrowHelper::init(&mut router, false);

    let config = helper.amp_query_config(&mut router).unwrap();
    assert_eq!(config.validators_limit, 30);

    let result = helper
        .amp_execute_sender(
            &mut router,
            ExecuteMsg::UpdateConfig {
                validators_limit: Some(40),
            },
            "user",
        )
        .unwrap_err();

    assert_eq!("Generic error: unauthorized", result.root_cause().to_string());

    helper
        .amp_execute(
            &mut router,
            ExecuteMsg::UpdateConfig {
                validators_limit: Some(40),
            },
        )
        .unwrap();

    let config = helper.amp_query_config(&mut router).unwrap();
    assert_eq!(config.validators_limit, 40);

    Ok(())
}

#[test]
fn vote() -> StdResult<()> {
    let mut router = mock_app();
    let helper = EscrowHelper::init(&mut router, false);

    helper.ve_lock_lp(&mut router, "user1", 100000, 3 * WEEK).unwrap();
    helper.ve_lock_lp(&mut router, "user2", 50000, 104 * WEEK).unwrap();

    let vote = helper.amp_vote(&mut router, "user1", vec![("val1".to_string(), 10000)]).unwrap();
    vote.assert_attribute("wasm", attr("vAMP", "125959")).unwrap();

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    let info = helper.amp_query_tune_info(&mut router).unwrap();
    assert_eq!(info.vamp_points, vec![("val1".to_string(), Uint128::new(125959))]);

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    let info = helper.amp_query_tune_info(&mut router).unwrap();
    assert_eq!(info.vamp_points, vec![("val1".to_string(), Uint128::new(117306))]);

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    let info = helper.amp_query_tune_info(&mut router).unwrap();
    assert_eq!(info.vamp_points, vec![("val1".to_string(), Uint128::new(108653))]);

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    let info = helper.amp_query_tune_info(&mut router).unwrap();
    assert_eq!(info.vamp_points, vec![("val1".to_string(), Uint128::new(100000))]);

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    let info = helper.amp_query_tune_info(&mut router).unwrap();
    assert_eq!(info.vamp_points, vec![("val1".to_string(), Uint128::new(100000))]);

    let vote = helper
        .amp_vote(
            &mut router,
            "user2",
            vec![("val1".to_string(), 3000), ("val2".to_string(), 7000)],
        )
        .unwrap();
    vote.assert_attribute("wasm", attr("vAMP", "478274")).unwrap();

    // vote is only applied in the next period
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    let info = helper.amp_query_tune_info(&mut router).unwrap();
    assert_eq!(info.vamp_points, vec![("val1".to_string(), Uint128::new(100000)),]);

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    let info = helper.amp_query_tune_info(&mut router).unwrap();
    assert_eq!(
        info.vamp_points,
        vec![
            ("val2".to_string(), Uint128::new(334791)), // ~ 446 * 0.7
            ("val1".to_string(), Uint128::new(243482))
        ]
    );

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    let info = helper.amp_query_tune_info(&mut router).unwrap();
    assert_eq!(
        info.vamp_points,
        vec![
            ("val2".to_string(), Uint128::new(331763)), // ~ 446 * 0.7 - decaying
            ("val1".to_string(), Uint128::new(242185))  //
        ]
    );

    router.next_period(105);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    let info = helper.amp_query_tune_info(&mut router).unwrap();
    assert_eq!(
        info.vamp_points,
        vec![
            ("val1".to_string(), Uint128::new(115079)), // rounding difference
            ("val2".to_string(), Uint128::new(35019))   // rounding difference
        ]
    );

    let result = helper.ve_withdraw(&mut router, "user1").unwrap();
    result.assert_attribute("wasm", attr("action", "vamp/update_vote_removed")).unwrap();

    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    let info = helper.amp_query_tune_info(&mut router).unwrap();
    assert_eq!(
        info.vamp_points,
        vec![
            ("val1".to_string(), Uint128::new(115079)), // rounding difference
            ("val2".to_string(), Uint128::new(35019))   // rounding difference
        ]
    );
    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    let info = helper.amp_query_tune_info(&mut router).unwrap();
    assert_eq!(
        info.vamp_points,
        vec![
            ("val2".to_string(), Uint128::new(35019)), // rounding difference
            ("val1".to_string(), Uint128::new(15079))  // rounding difference
        ]
    );
    Ok(())
}

#[test]
fn update_vote_extend_locktime() -> StdResult<()> {
    let mut router = mock_app();
    let helper = EscrowHelper::init(&mut router, false);

    helper.ve_lock_lp(&mut router, "user1", 100000, 3 * WEEK).unwrap();

    let vote = helper
        .amp_vote(
            &mut router,
            "user1",
            vec![
                ("val1".to_string(), 4000),
                ("val2".to_string(), 4000),
                ("val3".to_string(), 2000),
            ],
        )
        .unwrap();
    vote.assert_attribute("wasm", attr("vAMP", "125959")).unwrap();

    let err = helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap_err();
    assert_eq!(err.root_cause().to_string(), "There are no validators to tune");
    let info = helper.amp_query_tune_info(&mut router).unwrap();
    assert_eq!(info.vamp_points, vec![]);

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    let info = helper.amp_query_tune_info(&mut router).unwrap();
    assert_eq!(
        info.vamp_points,
        vec![
            ("val1".to_string(), Uint128::new(50383)),
            ("val2".to_string(), Uint128::new(50383)),
            ("val3".to_string(), Uint128::new(25191))
        ]
    );

    helper.ve_extend_lock_time(&mut router, "user1", 10).unwrap();
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    let info = helper.amp_query_tune_info(&mut router).unwrap();
    assert_eq!(
        info.vamp_points,
        vec![
            ("val1".to_string(), Uint128::new(50383)),
            ("val2".to_string(), Uint128::new(50383)),
            ("val3".to_string(), Uint128::new(25191))
        ]
    );

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    let info = helper.amp_query_tune_info(&mut router).unwrap();
    assert_eq!(
        info.vamp_points,
        vec![
            ("val1".to_string(), Uint128::new(81534)),
            ("val2".to_string(), Uint128::new(81534)),
            ("val3".to_string(), Uint128::new(40767))
        ]
    );

    Ok(())
}

#[test]
fn update_vote_extend_amount() -> StdResult<()> {
    let mut router = mock_app();
    let helper = EscrowHelper::init(&mut router, false);

    helper.ve_lock_lp(&mut router, "user1", 100000, 3 * WEEK).unwrap();

    let vote = helper
        .amp_vote(
            &mut router,
            "user1",
            vec![
                ("val1".to_string(), 4000),
                ("val2".to_string(), 4000),
                ("val3".to_string(), 2000),
            ],
        )
        .unwrap();
    vote.assert_attribute("wasm", attr("vAMP", "125959")).unwrap();

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    let info = helper.amp_query_tune_info(&mut router).unwrap();
    assert_eq!(
        info.vamp_points,
        vec![
            ("val1".to_string(), Uint128::new(50383)),
            ("val2".to_string(), Uint128::new(50383)),
            ("val3".to_string(), Uint128::new(25191))
        ]
    );

    helper.ve_add_funds_lock(&mut router, "user1", 1000000, Some(true)).unwrap();
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    let info = helper.amp_query_tune_info(&mut router).unwrap();
    assert_eq!(
        info.vamp_points,
        vec![
            ("val1".to_string(), Uint128::new(50383)),
            ("val2".to_string(), Uint128::new(50383)),
            ("val3".to_string(), Uint128::new(25191))
        ]
    );

    // cant withdraw before lock is up
    helper.ve_withdraw(&mut router, "user1").unwrap_err();

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    let info = helper.amp_query_tune_info(&mut router).unwrap();
    assert_eq!(
        info.vamp_points,
        vec![
            ("val1".to_string(), Uint128::new(554230)),
            ("val2".to_string(), Uint128::new(554230)),
            ("val3".to_string(), Uint128::new(277115))
        ]
    );

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    let info = helper.amp_query_tune_info(&mut router).unwrap();
    assert_eq!(
        info.vamp_points,
        vec![
            ("val1".to_string(), Uint128::new(516154)),
            ("val2".to_string(), Uint128::new(516154)),
            ("val3".to_string(), Uint128::new(258077))
        ]
    );

    router.next_period(1);
    helper.ve_withdraw(&mut router, "user1").unwrap();
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    let info = helper.amp_query_tune_info(&mut router).unwrap();
    assert_eq!(
        info.vamp_points,
        vec![
            ("val1".to_string(), Uint128::new(478078)),
            ("val2".to_string(), Uint128::new(478078)),
            ("val3".to_string(), Uint128::new(239039))
        ]
    );

    router.next_period(1);

    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    let info = helper.amp_query_tune_info(&mut router).unwrap();
    assert_eq!(
        info.vamp_points,
        vec![
            ("val1".to_string(), Uint128::new(2)),
            ("val2".to_string(), Uint128::new(2)),
            ("val3".to_string(), Uint128::new(1))
        ]
    );

    // let err = helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap_err();
    // assert_eq!(err.root_cause().to_string(), "There are no validators to tune");
    Ok(())
}

#[test]
fn check_update_owner() -> StdResult<()> {
    let mut router = mock_app();
    let helper = EscrowHelper::init(&mut router, false);

    let new_owner = String::from("new_owner");

    // New owner
    let msg = ExecuteMsg::ProposeNewOwner {
        new_owner: new_owner.clone(),
        expires_in: 100, // seconds
    };

    // Unauthed check
    let err = helper.amp_execute_sender(&mut router, msg.clone(), "not_owner").unwrap_err();

    assert_eq!(err.root_cause().to_string(), "Generic error: Unauthorized");

    // Claim before proposal
    let err = helper
        .amp_execute_sender(&mut router, ExecuteMsg::ClaimOwnership {}, new_owner.clone())
        .unwrap_err();
    assert_eq!(err.root_cause().to_string(), "Generic error: Ownership proposal not found");

    // Propose new owner
    helper.amp_execute_sender(&mut router, msg, "owner").unwrap();

    // Claim from invalid addr
    let err = helper
        .amp_execute_sender(&mut router, ExecuteMsg::ClaimOwnership {}, "invalid_addr")
        .unwrap_err();

    assert_eq!(err.root_cause().to_string(), "Generic error: Unauthorized");

    // Claim ownership
    helper
        .amp_execute_sender(&mut router, ExecuteMsg::ClaimOwnership {}, new_owner.clone())
        .unwrap();

    // Let's query the contract state
    let res: ConfigResponse = helper.amp_query_config(&mut router).unwrap();

    assert_eq!(res.owner, new_owner);
    Ok(())
}
