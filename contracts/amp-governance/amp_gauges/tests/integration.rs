use anyhow::{Ok, Result};
use cosmwasm_std::{attr, Addr, Uint128};
use eris::governance_helper::WEEK;
use eris_tests::escrow_helper::EscrowHelper;
use eris_tests::{mock_app, EventChecker, TerraAppExtension};
use std::vec;

use eris::amp_gauges::{ConfigResponse, ExecuteMsg};

#[test]
fn update_configs() -> Result<()> {
    let mut router = mock_app();
    let helper = EscrowHelper::init(&mut router);

    let config = helper.amp_query_config(&mut router)?;
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

    helper.amp_execute(
        &mut router,
        ExecuteMsg::UpdateConfig {
            validators_limit: Some(40),
        },
    )?;

    let config = helper.amp_query_config(&mut router)?;
    assert_eq!(config.validators_limit, 40);

    Ok(())
}

#[test]
fn vote() -> Result<()> {
    let mut router = mock_app();
    let helper = EscrowHelper::init(&mut router);

    helper.ve_lock(&mut router, "user1", 100000, 3 * WEEK).unwrap();
    helper.ve_lock(&mut router, "user2", 50000, 104 * WEEK).unwrap();

    let vote = helper.amp_vote(&mut router, "user1", vec![("val1".to_string(), 10000)])?;
    vote.assert_attribute("wasm", attr("vAMP", "223075"))?;

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {})?;
    let info = helper.amp_query_tune_info(&mut router)?;
    assert_eq!(info.vamp_points, vec![(Addr::unchecked("val1"), Uint128::new(223075))]);

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {})?;
    let info = helper.amp_query_tune_info(&mut router)?;
    assert_eq!(info.vamp_points, vec![(Addr::unchecked("val1"), Uint128::new(182050))]);

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {})?;
    let info = helper.amp_query_tune_info(&mut router)?;
    assert_eq!(info.vamp_points, vec![(Addr::unchecked("val1"), Uint128::new(141025))]);

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {})?;
    let info = helper.amp_query_tune_info(&mut router)?;
    assert_eq!(info.vamp_points, vec![(Addr::unchecked("val1"), Uint128::new(100000))]);

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {})?;
    let info = helper.amp_query_tune_info(&mut router)?;
    assert_eq!(info.vamp_points, vec![(Addr::unchecked("val1"), Uint128::new(100000))]);

    let vote = helper.amp_vote(
        &mut router,
        "user2",
        vec![("val1".to_string(), 3000), ("val2".to_string(), 7000)],
    )?;
    vote.assert_attribute("wasm", attr("vAMP", "478274"))?;

    // vote is only applied in the next period
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {})?;
    let info = helper.amp_query_tune_info(&mut router)?;
    assert_eq!(info.vamp_points, vec![(Addr::unchecked("val1"), Uint128::new(100000)),]);

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {})?;
    let info = helper.amp_query_tune_info(&mut router)?;
    assert_eq!(
        info.vamp_points,
        vec![
            (Addr::unchecked("val2"), Uint128::new(334791)), // ~ 446 * 0.7
            (Addr::unchecked("val1"), Uint128::new(243482))
        ]
    );

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {})?;
    let info = helper.amp_query_tune_info(&mut router)?;
    assert_eq!(
        info.vamp_points,
        vec![
            (Addr::unchecked("val2"), Uint128::new(331763)), // ~ 446 * 0.7 - decaying
            (Addr::unchecked("val1"), Uint128::new(242185))  //
        ]
    );

    router.next_period(105);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {})?;
    let info = helper.amp_query_tune_info(&mut router)?;
    assert_eq!(
        info.vamp_points,
        vec![
            (Addr::unchecked("val1"), Uint128::new(115079)), // rounding difference
            (Addr::unchecked("val2"), Uint128::new(35019))   // rounding difference
        ]
    );

    let result = helper.ve_withdraw(&mut router, "user1")?;
    result.assert_attribute("wasm", attr("action", "update_vote_removed"))?;

    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {})?;
    let info = helper.amp_query_tune_info(&mut router)?;
    assert_eq!(
        info.vamp_points,
        vec![
            (Addr::unchecked("val1"), Uint128::new(115079)), // rounding difference
            (Addr::unchecked("val2"), Uint128::new(35019))   // rounding difference
        ]
    );
    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {})?;
    let info = helper.amp_query_tune_info(&mut router)?;
    assert_eq!(
        info.vamp_points,
        vec![
            (Addr::unchecked("val2"), Uint128::new(35019)), // rounding difference
            (Addr::unchecked("val1"), Uint128::new(15079))  // rounding difference
        ]
    );
    Ok(())
}

#[test]
fn update_vote_extend_locktime() -> Result<()> {
    let mut router = mock_app();
    let helper = EscrowHelper::init(&mut router);

    helper.ve_lock(&mut router, "user1", 100000, 3 * WEEK)?;

    let vote = helper.amp_vote(
        &mut router,
        "user1",
        vec![("val1".to_string(), 4000), ("val2".to_string(), 4000), ("val3".to_string(), 2000)],
    )?;
    vote.assert_attribute("wasm", attr("vAMP", "223075"))?;

    let err = helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap_err();
    assert_eq!(err.root_cause().to_string(), "There are no validators to tune");
    let info = helper.amp_query_tune_info(&mut router)?;
    assert_eq!(info.vamp_points, vec![]);

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {})?;
    let info = helper.amp_query_tune_info(&mut router)?;
    assert_eq!(
        info.vamp_points,
        vec![
            (Addr::unchecked("val1"), Uint128::new(89230)),
            (Addr::unchecked("val2"), Uint128::new(89230)),
            (Addr::unchecked("val3"), Uint128::new(44615))
        ]
    );

    helper.ve_extend_lock_time(&mut router, "user1", 10)?;
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {})?;
    let info = helper.amp_query_tune_info(&mut router)?;
    assert_eq!(
        info.vamp_points,
        vec![
            (Addr::unchecked("val1"), Uint128::new(89230)),
            (Addr::unchecked("val2"), Uint128::new(89230)),
            (Addr::unchecked("val3"), Uint128::new(44615))
        ]
    );

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {})?;
    let info = helper.amp_query_tune_info(&mut router)?;
    assert_eq!(
        info.vamp_points,
        vec![
            (Addr::unchecked("val1"), Uint128::new(116920)),
            (Addr::unchecked("val2"), Uint128::new(116920)),
            (Addr::unchecked("val3"), Uint128::new(58460))
        ]
    );

    Ok(())
}

#[test]
fn update_vote_extend_amount() -> Result<()> {
    let mut router = mock_app();
    let helper = EscrowHelper::init(&mut router);

    helper.ve_lock(&mut router, "user1", 100000, 3 * WEEK)?;

    let vote = helper.amp_vote(
        &mut router,
        "user1",
        vec![("val1".to_string(), 4000), ("val2".to_string(), 4000), ("val3".to_string(), 2000)],
    )?;
    vote.assert_attribute("wasm", attr("vAMP", "223075"))?;

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {})?;
    let info = helper.amp_query_tune_info(&mut router)?;
    assert_eq!(
        info.vamp_points,
        vec![
            (Addr::unchecked("val1"), Uint128::new(89230)),
            (Addr::unchecked("val2"), Uint128::new(89230)),
            (Addr::unchecked("val3"), Uint128::new(44615))
        ]
    );

    helper.ve_add_funds_lock(&mut router, "user1", 1000000)?;
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {})?;
    let info = helper.amp_query_tune_info(&mut router)?;
    assert_eq!(
        info.vamp_points,
        vec![
            (Addr::unchecked("val1"), Uint128::new(89230)),
            (Addr::unchecked("val2"), Uint128::new(89230)),
            (Addr::unchecked("val3"), Uint128::new(44615))
        ]
    );

    // cant withdraw before lock is up
    helper.ve_withdraw(&mut router, "user1").unwrap_err();

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {})?;
    let info = helper.amp_query_tune_info(&mut router)?;
    assert_eq!(
        info.vamp_points,
        vec![
            (Addr::unchecked("val1"), Uint128::new(934358)),
            (Addr::unchecked("val2"), Uint128::new(934358)),
            (Addr::unchecked("val3"), Uint128::new(467179))
        ]
    );

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {})?;
    let info = helper.amp_query_tune_info(&mut router)?;
    assert_eq!(
        info.vamp_points,
        vec![
            (Addr::unchecked("val1"), Uint128::new(687179)),
            (Addr::unchecked("val2"), Uint128::new(687179)),
            (Addr::unchecked("val3"), Uint128::new(343590))
        ]
    );

    helper.ve_withdraw(&mut router, "user1")?;
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {})?;
    let info = helper.amp_query_tune_info(&mut router)?;
    assert_eq!(
        info.vamp_points,
        vec![
            (Addr::unchecked("val1"), Uint128::new(687179)),
            (Addr::unchecked("val2"), Uint128::new(687179)),
            (Addr::unchecked("val3"), Uint128::new(343590))
        ]
    );

    router.next_period(1);

    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {})?;
    let info = helper.amp_query_tune_info(&mut router)?;
    assert_eq!(info.vamp_points, vec![(Addr::unchecked("val3"), Uint128::new(1)),]);
    Ok(())
}

#[test]
fn check_update_owner() -> Result<()> {
    let mut router = mock_app();
    let helper = EscrowHelper::init(&mut router);

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
    helper.amp_execute_sender(&mut router, msg, "owner")?;

    // Claim from invalid addr
    let err = helper
        .amp_execute_sender(&mut router, ExecuteMsg::ClaimOwnership {}, "invalid_addr")
        .unwrap_err();

    assert_eq!(err.root_cause().to_string(), "Generic error: Unauthorized");

    // Claim ownership
    helper.amp_execute_sender(&mut router, ExecuteMsg::ClaimOwnership {}, new_owner.clone())?;

    // Let's query the contract state
    let res: ConfigResponse = helper.amp_query_config(&mut router)?;

    assert_eq!(res.owner, new_owner);
    Ok(())
}
