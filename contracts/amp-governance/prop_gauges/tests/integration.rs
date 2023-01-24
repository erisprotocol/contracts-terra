use cosmwasm_std::{attr, StdResult};
use eris::governance_helper::WEEK;
use eris_tests::gov_helper::EscrowHelper;
use eris_tests::{mock_app, EventChecker, TerraAppExtension};

use eris::prop_gauges::ExecuteMsg;

#[test]
fn update_configs() -> StdResult<()> {
    let mut router = mock_app();
    let helper = EscrowHelper::init(&mut router, false);

    let config = helper.prop_query_config(&mut router).unwrap();
    assert_eq!(config.quorum_bps, 500u16);

    let result = helper
        .prop_execute_sender(
            &mut router,
            ExecuteMsg::UpdateConfig {
                quorum_bps: Some(100u16),
                use_weighted_vote: None,
            },
            "user",
        )
        .unwrap_err();

    assert_eq!("Generic error: unauthorized", result.root_cause().to_string());

    helper
        .prop_execute(
            &mut router,
            ExecuteMsg::UpdateConfig {
                quorum_bps: Some(100u16),
                use_weighted_vote: None,
            },
        )
        .unwrap();

    let config = helper.prop_query_config(&mut router).unwrap();
    assert_eq!(config.quorum_bps, 100u16);

    Ok(())
}

// #[test]
fn _vote() -> StdResult<()> {
    let mut router = mock_app();
    let helper = EscrowHelper::init(&mut router, false);
    let router_ref = &mut router;

    helper.ve_lock_lp(router_ref, "user1", 4000, 104 * WEEK).unwrap();
    helper.ve_lock_lp(router_ref, "user2", 4000, 104 * WEEK).unwrap();
    helper.ve_lock_lp(router_ref, "user3", 92000, 104 * WEEK).unwrap();

    // VOTE WITHOUT PROP
    let result =
        helper.prop_vote(router_ref, "user1", 10, cosmwasm_std::VoteOption::Yes).unwrap_err();
    assert_eq!(
        "Generic error: proposal with id 10 not initialized",
        result.root_cause().to_string()
    );

    // INIT THE PROPOSAL
    let result = helper
        .prop_init(router_ref, "user1", 10, router_ref.block_info().time.seconds() + 100)
        .unwrap_err();
    assert_eq!("Generic error: unauthorized", result.root_cause().to_string());

    let result = helper
        .prop_init(router_ref, "owner", 10, router_ref.block_info().time.seconds() + 3 * WEEK)
        .unwrap();

    result.assert_attribute("wasm", attr("action", "prop/init_prop")).unwrap();
    result.assert_attribute("wasm", attr("prop", "10")).unwrap();
    result.assert_attribute("wasm", attr("end", "3")).unwrap();

    helper
        .prop_init(router_ref, "owner", 11, router_ref.block_info().time.seconds() + 3 * WEEK)
        .unwrap();
    helper
        .prop_init(router_ref, "owner", 12, router_ref.block_info().time.seconds() + 3 * WEEK)
        .unwrap();

    // VOTE USER1
    let result = helper.prop_vote(router_ref, "user1", 10, cosmwasm_std::VoteOption::Yes).unwrap();
    result.assert_attribute("wasm", attr("action", "prop/vote")).unwrap();
    result.assert_attribute("wasm", attr("vp", "38946")).unwrap();

    // no matter the period, it is always the same VP (using different props, but same setup)
    router_ref.next_period(1);
    // VOTE USER1
    let result = helper.prop_vote(router_ref, "user1", 11, cosmwasm_std::VoteOption::Yes).unwrap();
    result.assert_attribute("wasm", attr("action", "prop/vote")).unwrap();
    result.assert_attribute("wasm", attr("vp", "38946")).unwrap();

    router_ref.next_period(1);
    // VOTE USER1
    let result = helper.prop_vote(router_ref, "user1", 12, cosmwasm_std::VoteOption::Yes).unwrap();
    result.assert_attribute("wasm", attr("action", "prop/vote")).unwrap();
    result.assert_attribute("wasm", attr("vp", "38946")).unwrap();

    // VOTE USER2
    let result =
        helper.prop_vote(router_ref, "user2", 10, cosmwasm_std::VoteOption::Yes).unwrap_err();
    assert_eq!("No vote operator set", result.root_cause().to_string());

    helper
        .hub_execute(
            router_ref,
            eris::hub::ExecuteMsg::UpdateConfig {
                protocol_fee_contract: None,
                protocol_reward_fee: None,
                delegation_strategy: None,
                allow_donations: None,
                vote_operator: Some(helper.base.prop_gauges.get_address_string()),
            },
        )
        .unwrap();

    let result = helper.prop_vote(router_ref, "user2", 10, cosmwasm_std::VoteOption::Yes).unwrap();
    result.assert_attribute("wasm", attr("action", "prop/vote")).unwrap();
    result.assert_attribute("wasm", attr("vp", "38946")).unwrap();

    // res.assert_attribute("wasm", attr("xx", "12")).unwrap();

    // let vote = helper.amp_vote(&mut router, "user1", vec![("val1".to_string(), 10000)]).unwrap();
    // vote.assert_attribute("wasm", attr("vAMP", "125959")).unwrap();

    // router.next_period(1);
    // helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    // let info = helper.amp_query_tune_info(&mut router).unwrap();
    // assert_eq!(info.vamp_points, vec![("val1".to_string(), Uint128::new(125959))]);

    // router.next_period(1);
    // helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    // let info = helper.amp_query_tune_info(&mut router).unwrap();
    // assert_eq!(info.vamp_points, vec![("val1".to_string(), Uint128::new(117306))]);

    // router.next_period(1);
    // helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    // let info = helper.amp_query_tune_info(&mut router).unwrap();
    // assert_eq!(info.vamp_points, vec![("val1".to_string(), Uint128::new(108653))]);

    // router.next_period(1);
    // helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    // let info = helper.amp_query_tune_info(&mut router).unwrap();
    // assert_eq!(info.vamp_points, vec![("val1".to_string(), Uint128::new(100000))]);

    // router.next_period(1);
    // helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    // let info = helper.amp_query_tune_info(&mut router).unwrap();
    // assert_eq!(info.vamp_points, vec![("val1".to_string(), Uint128::new(100000))]);

    // let vote = helper
    //     .amp_vote(
    //         &mut router,
    //         "user2",
    //         vec![("val1".to_string(), 3000), ("val2".to_string(), 7000)],
    //     )
    //     .unwrap();
    // vote.assert_attribute("wasm", attr("vAMP", "478274")).unwrap();

    // // vote is only applied in the next period
    // helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    // let info = helper.amp_query_tune_info(&mut router).unwrap();
    // assert_eq!(info.vamp_points, vec![("val1".to_string(), Uint128::new(100000)),]);

    // router.next_period(1);
    // helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    // let info = helper.amp_query_tune_info(&mut router).unwrap();
    // assert_eq!(
    //     info.vamp_points,
    //     vec![
    //         ("val2".to_string(), Uint128::new(334791)), // ~ 446 * 0.7
    //         ("val1".to_string(), Uint128::new(243482))
    //     ]
    // );

    // router.next_period(1);
    // helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    // let info = helper.amp_query_tune_info(&mut router).unwrap();
    // assert_eq!(
    //     info.vamp_points,
    //     vec![
    //         ("val2".to_string(), Uint128::new(331763)), // ~ 446 * 0.7 - decaying
    //         ("val1".to_string(), Uint128::new(242185))  //
    //     ]
    // );

    // router.next_period(105);
    // helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    // let info = helper.amp_query_tune_info(&mut router).unwrap();
    // assert_eq!(
    //     info.vamp_points,
    //     vec![
    //         ("val1".to_string(), Uint128::new(115079)), // rounding difference
    //         ("val2".to_string(), Uint128::new(35019))   // rounding difference
    //     ]
    // );

    // let result = helper.ve_withdraw(&mut router, "user1").unwrap();
    // result.assert_attribute("wasm", attr("action", "vamp/update_vote_removed")).unwrap();

    // helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    // let info = helper.amp_query_tune_info(&mut router).unwrap();
    // assert_eq!(
    //     info.vamp_points,
    //     vec![
    //         ("val1".to_string(), Uint128::new(115079)), // rounding difference
    //         ("val2".to_string(), Uint128::new(35019))   // rounding difference
    //     ]
    // );
    // router.next_period(1);
    // helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    // let info = helper.amp_query_tune_info(&mut router).unwrap();
    // assert_eq!(
    //     info.vamp_points,
    //     vec![
    //         ("val2".to_string(), Uint128::new(35019)), // rounding difference
    //         ("val1".to_string(), Uint128::new(15079))  // rounding difference
    //     ]
    // );
    Ok(())
}

// #[test]
// fn update_vote_extend_locktime() -> StdResult<()> {
//     let mut router = mock_app();
//     let helper = EscrowHelper::init(&mut router, false);

//     helper.ve_lock_lp(&mut router, "user1", 100000, 3 * WEEK).unwrap();

//     let vote = helper
//         .amp_vote(
//             &mut router,
//             "user1",
//             vec![
//                 ("val1".to_string(), 4000),
//                 ("val2".to_string(), 4000),
//                 ("val3".to_string(), 2000),
//             ],
//         )
//         .unwrap();
//     vote.assert_attribute("wasm", attr("vAMP", "125959")).unwrap();

//     let err = helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap_err();
//     assert_eq!(err.root_cause().to_string(), "There are no validators to tune");
//     let info = helper.amp_query_tune_info(&mut router).unwrap();
//     assert_eq!(info.vamp_points, vec![]);

//     router.next_period(1);
//     helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
//     let info = helper.amp_query_tune_info(&mut router).unwrap();
//     assert_eq!(
//         info.vamp_points,
//         vec![
//             ("val1".to_string(), Uint128::new(50383)),
//             ("val2".to_string(), Uint128::new(50383)),
//             ("val3".to_string(), Uint128::new(25191))
//         ]
//     );

//     helper.ve_extend_lock_time(&mut router, "user1", 10).unwrap();
//     helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
//     let info = helper.amp_query_tune_info(&mut router).unwrap();
//     assert_eq!(
//         info.vamp_points,
//         vec![
//             ("val1".to_string(), Uint128::new(50383)),
//             ("val2".to_string(), Uint128::new(50383)),
//             ("val3".to_string(), Uint128::new(25191))
//         ]
//     );

//     router.next_period(1);
//     helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
//     let info = helper.amp_query_tune_info(&mut router).unwrap();
//     assert_eq!(
//         info.vamp_points,
//         vec![
//             ("val1".to_string(), Uint128::new(81534)),
//             ("val2".to_string(), Uint128::new(81534)),
//             ("val3".to_string(), Uint128::new(40767))
//         ]
//     );

//     Ok(())
// }

// #[test]
// fn update_vote_extend_amount() -> StdResult<()> {
//     let mut router = mock_app();
//     let helper = EscrowHelper::init(&mut router, false);

//     helper.ve_lock_lp(&mut router, "user1", 100000, 3 * WEEK).unwrap();

//     let vote = helper
//         .amp_vote(
//             &mut router,
//             "user1",
//             vec![
//                 ("val1".to_string(), 4000),
//                 ("val2".to_string(), 4000),
//                 ("val3".to_string(), 2000),
//             ],
//         )
//         .unwrap();
//     vote.assert_attribute("wasm", attr("vAMP", "125959")).unwrap();

//     router.next_period(1);
//     helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
//     let info = helper.amp_query_tune_info(&mut router).unwrap();
//     assert_eq!(
//         info.vamp_points,
//         vec![
//             ("val1".to_string(), Uint128::new(50383)),
//             ("val2".to_string(), Uint128::new(50383)),
//             ("val3".to_string(), Uint128::new(25191))
//         ]
//     );

//     helper.ve_add_funds_lock(&mut router, "user1", 1000000).unwrap();
//     helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
//     let info = helper.amp_query_tune_info(&mut router).unwrap();
//     assert_eq!(
//         info.vamp_points,
//         vec![
//             ("val1".to_string(), Uint128::new(50383)),
//             ("val2".to_string(), Uint128::new(50383)),
//             ("val3".to_string(), Uint128::new(25191))
//         ]
//     );

//     // cant withdraw before lock is up
//     helper.ve_withdraw(&mut router, "user1").unwrap_err();

//     router.next_period(1);
//     helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
//     let info = helper.amp_query_tune_info(&mut router).unwrap();
//     assert_eq!(
//         info.vamp_points,
//         vec![
//             ("val1".to_string(), Uint128::new(516152)),
//             ("val2".to_string(), Uint128::new(516152)),
//             ("val3".to_string(), Uint128::new(258076))
//         ]
//     );

//     router.next_period(1);
//     helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
//     let info = helper.amp_query_tune_info(&mut router).unwrap();
//     assert_eq!(
//         info.vamp_points,
//         vec![
//             ("val1".to_string(), Uint128::new(478076)),
//             ("val2".to_string(), Uint128::new(478076)),
//             ("val3".to_string(), Uint128::new(239038))
//         ]
//     );

//     helper.ve_withdraw(&mut router, "user1").unwrap();
//     helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
//     let info = helper.amp_query_tune_info(&mut router).unwrap();
//     assert_eq!(
//         info.vamp_points,
//         vec![
//             ("val1".to_string(), Uint128::new(478076)),
//             ("val2".to_string(), Uint128::new(478076)),
//             ("val3".to_string(), Uint128::new(239038))
//         ]
//     );

//     router.next_period(1);

//     let err = helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap_err();
//     assert_eq!(err.root_cause().to_string(), "There are no validators to tune");
//     Ok(())
// }

// #[test]
// fn check_update_owner() -> StdResult<()> {
//     let mut router = mock_app();
//     let helper = EscrowHelper::init(&mut router, false);

//     let new_owner = String::from("new_owner");

//     // New owner
//     let msg = ExecuteMsg::ProposeNewOwner {
//         new_owner: new_owner.clone(),
//         expires_in: 100, // seconds
//     };

//     // Unauthed check
//     let err = helper.amp_execute_sender(&mut router, msg.clone(), "not_owner").unwrap_err();

//     assert_eq!(err.root_cause().to_string(), "Generic error: Unauthorized");

//     // Claim before proposal
//     let err = helper
//         .amp_execute_sender(&mut router, ExecuteMsg::ClaimOwnership {}, new_owner.clone())
//         .unwrap_err();
//     assert_eq!(err.root_cause().to_string(), "Generic error: Ownership proposal not found");

//     // Propose new owner
//     helper.amp_execute_sender(&mut router, msg, "owner").unwrap();

//     // Claim from invalid addr
//     let err = helper
//         .amp_execute_sender(&mut router, ExecuteMsg::ClaimOwnership {}, "invalid_addr")
//         .unwrap_err();

//     assert_eq!(err.root_cause().to_string(), "Generic error: Unauthorized");

//     // Claim ownership
//     helper
//         .amp_execute_sender(&mut router, ExecuteMsg::ClaimOwnership {}, new_owner.clone())
//         .unwrap();

//     // Let's query the contract state
//     let res: ConfigResponse = helper.amp_query_config(&mut router).unwrap();

//     assert_eq!(res.owner, new_owner);
//     Ok(())
// }
