use amp_gauges::state::VotedValidatorInfo;
use anyhow::{Ok, Result};
use cosmwasm_std::{attr, Addr, Uint128};
use eris::governance_helper::WEEK;
use eris_tests::escrow_helper::EscrowHelper;
use eris_tests::{mock_app, EventChecker, TerraAppExtension};
use std::vec;

use eris::amp_gauges::{ConfigResponse, ExecuteMsg, GaugeInfoResponse};

#[test]
fn update_configs() {
    let mut router = mock_app();
    let helper = EscrowHelper::init(&mut router);

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
}

#[test]
fn vote() -> Result<()> {
    let mut router = mock_app();
    let helper = EscrowHelper::init(&mut router);

    helper.ve_lock(&mut router, "user1", 100, 2 * WEEK)?;
    helper.ve_lock(&mut router, "user2", 50, 100 * WEEK)?;

    let vote = helper.amp_vote(&mut router, "user1", vec![("val1".to_string(), 10000)])?;
    vote.assert_attribute("wasm", attr("vAMP", "102"))?;

    router.next_period(1);
    helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {})?;
    let info = helper.amp_query_tune_info(&mut router)?;
    assert_eq!(
        info,
        GaugeInfoResponse {
            tune_ts: 1667779200,
            vamp_points: vec![(Addr::unchecked("val1"), Uint128::new(102))]
        }
    );

    println!("---");

    router.next_period(1);
    let err = helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();

    println!("{:?}", err);

    let info = helper.amp_query_tune_info(&mut router).unwrap();
    assert_eq!(
        info,
        GaugeInfoResponse {
            tune_ts: 1667779200,
            vamp_points: vec![(Addr::unchecked("val1"), Uint128::new(101))]
        }
    );

    // let vote = helper
    //     .amp_vote(
    //         &mut router,
    //         "user2",
    //         vec![("val1".to_string(), 3000), ("val2".to_string(), 7000)],
    //     )
    //     ?;
    // vote.assert_attribute("wasm", attr("vAMP", "122"))?;

    // router.next_period(1);
    // helper.amp_execute(&mut router, ExecuteMsg::TuneVamp {}).unwrap();
    // let info = helper.amp_query_tune_info(&mut router).unwrap();
    // assert_eq!(
    //     info,
    //     GaugeInfoResponse {
    //         tune_ts: 1667779200,
    //         vamp_points: vec![
    //             (Addr::unchecked("val1"), Uint128::new(49)),
    //             (Addr::unchecked("val2"), Uint128::new(49))
    //         ]
    //     }
    // );

    Ok(())
}

// #[test]
// fn add_points() -> Result<()> {
//     let mut router = mock_app();
//     let helper = EscrowHelper::init(&mut router);

//     let result = helper
//         .emp_execute(
//             &mut router,
//             ExecuteMsg::AddEmps {
//                 emps: vec![(
//                     "unknown-validator".to_string(),
//                     vec![EmpInfo {
//                         decaying_period: Some(3),
//                         umerit_points: Uint128::new(1000000),
//                     }],
//                 )],
//             },
//         )
//         .unwrap_err();

//     assert_eq!("Invalid validator address: unknown-validator", result.root_cause().to_string());

//     let result = helper.emp_execute(
//         &mut router,
//         ExecuteMsg::AddEmps {
//             emps: vec![
//                 (
//                     "val1".to_string(),
//                     vec![
//                         EmpInfo {
//                             decaying_period: Some(2 * 4), // 2 months
//                             umerit_points: Uint128::new(2000000),
//                         },
//                         EmpInfo {
//                             decaying_period: None,
//                             umerit_points: Uint128::new(1000000),
//                         },
//                     ],
//                 ),
//                 (
//                     "val2".to_string(),
//                     vec![
//                         EmpInfo {
//                             decaying_period: Some(2 * 4), // 2 months
//                             umerit_points: Uint128::new(1000000),
//                         },
//                         EmpInfo {
//                             decaying_period: None,
//                             umerit_points: Uint128::new(2000000),
//                         },
//                     ],
//                 ),
//             ],
//         },
//     )?;
//     result.assert_attribute("wasm", attr("emps", "val1=3000000"))?;
//     result.assert_attribute("wasm", attr("emps", "val2=3000000"))?;

//     let old_period = router.block_period();
//     router.next_period(4);
//     let current_period = router.block_period();
//     assert_eq!(old_period + 4, current_period);

//     let result = helper.emp_execute(&mut router, ExecuteMsg::TuneEmps {}).unwrap();
//     result.assert_attribute("wasm", attr("emps", "val1=2000000"))?;
//     result.assert_attribute("wasm", attr("emps", "val2=2500000"))?;

//     let result = helper.emp_execute(
//         &mut router,
//         ExecuteMsg::AddEmps {
//             emps: vec![
//                 (
//                     "val3".to_string(),
//                     vec![EmpInfo {
//                         decaying_period: Some(4), // 1 months
//                         umerit_points: Uint128::new(1000000),
//                     }],
//                 ),
//                 (
//                     "val2".to_string(),
//                     vec![EmpInfo {
//                         decaying_period: Some(4), // 1 months
//                         umerit_points: Uint128::new(1000000),
//                     }],
//                 ),
//                 (
//                     "val4".to_string(),
//                     vec![EmpInfo {
//                         decaying_period: None,
//                         umerit_points: Uint128::new(500000),
//                     }],
//                 ),
//             ],
//         },
//     )?;
//     result.assert_attribute("wasm", attr("emps", "val1=2000000"))?;
//     result.assert_attribute("wasm", attr("emps", "val2=3500000"))?;
//     result.assert_attribute("wasm", attr("emps", "val3=1000000"))?;
//     result.assert_attribute("wasm", attr("emps", "val4=500000"))?;

//     let result = helper.emp_query_tune_info(&mut router)?;
//     assert_eq!(
//         result,
//         GaugeInfoResponse {
//             tune_ts: 1669593600,
//             emp_points: vec![
//                 (Addr::unchecked("val2"), Uint128::new(3500000)),
//                 (Addr::unchecked("val1"), Uint128::new(2000000)),
//                 (Addr::unchecked("val3"), Uint128::new(1000000)),
//                 (Addr::unchecked("val4"), Uint128::new(500000)),
//             ]
//         }
//     );

//     router.next_period(2);
//     let result = helper.emp_execute(&mut router, ExecuteMsg::TuneEmps {}).unwrap();
//     result.assert_attribute("wasm", attr("emps", "val1=1500000"))?;
//     result.assert_attribute("wasm", attr("emps", "val2=2750000"))?;
//     result.assert_attribute("wasm", attr("emps", "val3=500000"))?;
//     result.assert_attribute("wasm", attr("emps", "val4=500000"))?;
//     let result = helper.emp_execute(&mut router, ExecuteMsg::TuneEmps {}).unwrap();
//     result.assert_attribute("wasm", attr("emps", "val1=1500000"))?;
//     result.assert_attribute("wasm", attr("emps", "val2=2750000"))?;
//     result.assert_attribute("wasm", attr("emps", "val3=500000"))?;
//     result.assert_attribute("wasm", attr("emps", "val4=500000"))?;

//     router.next_period(2);
//     let result = helper.emp_execute(&mut router, ExecuteMsg::TuneEmps {}).unwrap();
//     result.assert_attribute("wasm", attr("emps", "val1=1000000"))?;
//     result.assert_attribute("wasm", attr("emps", "val2=2000000"))?;
//     result.assert_attribute("wasm", attr("emps", "val4=500000"))?;

//     router.next_period(4);
//     let result = helper.emp_execute(&mut router, ExecuteMsg::TuneEmps {}).unwrap();
//     result.assert_attribute("wasm", attr("emps", "val1=1000000"))?;
//     result.assert_attribute("wasm", attr("emps", "val2=2000000"))?;
//     result.assert_attribute("wasm", attr("emps", "val4=500000"))?;

//     let result = helper.emp_execute(&mut router, ExecuteMsg::TuneEmps {}).unwrap();
//     result.assert_attribute("wasm", attr("emps", "val1=1000000"))?;
//     result.assert_attribute("wasm", attr("emps", "val2=2000000"))?;
//     result.assert_attribute("wasm", attr("emps", "val4=500000"))?;

//     let result = helper.emp_query_validator_history(&mut router, "val1", 0)?;
//     assert_eq!(
//         result,
//         VotedValidatorInfo {
//             emp_amount: Uint128::new(3000000),
//             slope: Uint128::new(250000)
//         }
//     );

//     let result = helper.emp_query_validator_history(&mut router, "val1", 1)?;
//     assert_eq!(
//         result,
//         VotedValidatorInfo {
//             emp_amount: Uint128::new(2750000),
//             slope: Uint128::new(250000)
//         }
//     );

//     let result = helper.emp_query_validator_history(&mut router, "val4", 0)?;
//     assert_eq!(
//         result,
//         VotedValidatorInfo {
//             emp_amount: Uint128::zero(),
//             slope: Uint128::zero()
//         }
//     );

//     let result = helper.emp_query_validator_history(&mut router, "val4", 2)?;
//     assert_eq!(
//         result,
//         VotedValidatorInfo {
//             emp_amount: Uint128::zero(),
//             slope: Uint128::zero()
//         }
//     );

//     let result = helper.emp_query_validator_history(&mut router, "val4", 4)?;
//     assert_eq!(
//         result,
//         VotedValidatorInfo {
//             emp_amount: Uint128::new(500000),
//             slope: Uint128::zero()
//         }
//     );

//     let result = helper.emp_query_validator_history(&mut router, "val4", 5)?;
//     assert_eq!(
//         result,
//         VotedValidatorInfo {
//             emp_amount: Uint128::new(500000),
//             slope: Uint128::zero()
//         }
//     );

//     Ok(())
// }

// #[test]
// fn check_kick_holders_works() -> Result<()> {
//     let mut router = mock_app();
//     let helper = EscrowHelper::init(&mut router);

//     let result = helper.emp_execute(
//         &mut router,
//         ExecuteMsg::AddEmps {
//             emps: vec![
//                 (
//                     "val1".to_string(),
//                     vec![
//                         EmpInfo {
//                             decaying_period: Some(2 * 4), // 2 months
//                             umerit_points: Uint128::new(2000000),
//                         },
//                         EmpInfo {
//                             decaying_period: None,
//                             umerit_points: Uint128::new(1000000),
//                         },
//                     ],
//                 ),
//                 (
//                     "val2".to_string(),
//                     vec![
//                         EmpInfo {
//                             decaying_period: Some(2 * 4), // 2 months
//                             umerit_points: Uint128::new(1000000),
//                         },
//                         EmpInfo {
//                             decaying_period: None,
//                             umerit_points: Uint128::new(2000000),
//                         },
//                     ],
//                 ),
//             ],
//         },
//     )?;

//     result.assert_attribute("wasm", attr("emps", "val1=3000000"))?;
//     result.assert_attribute("wasm", attr("emps", "val2=3000000"))?;

//     helper.hub_remove_validator(&mut router, "val2")?;

//     let result = helper.emp_execute(&mut router, ExecuteMsg::TuneEmps {}).unwrap();
//     result.assert_attribute("wasm", attr("emps", "val1=3000000"))?;
//     let err = result.assert_attribute("wasm", attr("emps", "val2=3000000")).unwrap_err();
//     assert_eq!(err.to_string(), "Could not find key: emps value: val2=3000000");

//     Ok(())
// }

// #[test]
// fn check_update_owner() {
//     let mut router = mock_app();
//     let helper = EscrowHelper::init(&mut router);

//     let new_owner = String::from("new_owner");

//     // New owner
//     let msg = ExecuteMsg::ProposeNewOwner {
//         new_owner: new_owner.clone(),
//         expires_in: 100, // seconds
//     };

//     // Unauthed check
//     let err = helper.emp_execute_sender(&mut router, msg.clone(), "not_owner").unwrap_err();

//     assert_eq!(err.root_cause().to_string(), "Generic error: Unauthorized");

//     // Claim before proposal
//     let err = helper
//         .emp_execute_sender(&mut router, ExecuteMsg::ClaimOwnership {}, new_owner.clone())
//         .unwrap_err();
//     assert_eq!(err.root_cause().to_string(), "Generic error: Ownership proposal not found");

//     // Propose new owner
//     helper.emp_execute_sender(&mut router, msg, "owner").unwrap();

//     // Claim from invalid addr
//     let err = helper
//         .emp_execute_sender(&mut router, ExecuteMsg::ClaimOwnership {}, "invalid_addr")
//         .unwrap_err();

//     assert_eq!(err.root_cause().to_string(), "Generic error: Unauthorized");

//     // Claim ownership
//     helper
//         .emp_execute_sender(&mut router, ExecuteMsg::ClaimOwnership {}, new_owner.clone())
//         .unwrap();

//     // Let's query the contract state
//     let res: ConfigResponse = helper.emp_query_config(&mut router).unwrap();

//     assert_eq!(res.owner, new_owner)
// }
