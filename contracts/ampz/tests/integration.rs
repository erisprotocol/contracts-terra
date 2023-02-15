// use anyhow::{Ok, Result};
// use cosmwasm_std::{attr, Addr, Decimal, Uint128};
// use eris_tests::{gov_helper::EscrowHelper, TerraAppExtension};
// use eris_tests::{mock_app, mock_app_validators, EventChecker};
// use itertools::Itertools;
// use std::str::FromStr;
// use std::vec;

// use eris::{
//     emp_gauges::EmpInfo,
//     governance_helper::WEEK,
//     hub::{ConfigResponse, ExecuteMsg, FeeConfig},
// };

// #[test]
// fn update_configs() -> Result<()> {
//     let mut router = mock_app();
//     let helper = EscrowHelper::init(&mut router);

//     let config = helper.hub_query_config(&mut router)?;
//     assert_eq!(
//         config,
//         ConfigResponse {
//             owner: "owner".to_string(),
//             new_owner: None,
//             stake_token: helper.base.amp_token.get_address_string(),
//             epoch_period: 259200,
//             unbond_period: 1814400,
//             validators: vec![
//                 "val1".to_string(),
//                 "val2".to_string(),
//                 "val3".to_string(),
//                 "val4".to_string()
//             ],
//             fee_config: FeeConfig {
//                 protocol_fee_contract: Addr::unchecked("fee"),
//                 protocol_reward_fee: Decimal::from_ratio(1u128, 100u128)
//             },
//             delegation_strategy: eris::hub::DelegationStrategy::Gauges {
//                 amp_gauges: helper.base.amp_gauges.get_address_string(),
//                 emp_gauges: helper.base.emp_gauges.get_address_string(),
//                 amp_factor_bps: 5000,
//                 min_delegation_bps: 100,
//                 max_delegation_bps: 2500,
//                 validator_count: 5,
//             },
//         }
//     );

//     let result = helper
//         .hub_execute_sender(
//             &mut router,
//             ExecuteMsg::UpdateConfig {
//                 protocol_reward_fee: Some(Decimal::from_str("0")?),
//                 delegation_strategy: None,
//                 protocol_fee_contract: None,
//             },
//             Addr::unchecked("user"),
//         )
//         .unwrap_err();

//     assert_eq!("Generic error: unauthorized: sender is not owner", result.root_cause().to_string());

//     helper
//         .hub_execute(
//             &mut router,
//             ExecuteMsg::UpdateConfig {
//                 protocol_reward_fee: Some(Decimal::from_str("0.1")?),
//                 protocol_fee_contract: Some("fee_new".to_string()),
//                 delegation_strategy: Some(eris::hub::DelegationStrategy::Uniform),
//             },
//         )
//         .unwrap();

//     let config = helper.hub_query_config(&mut router)?;
//     assert_eq!(
//         config,
//         ConfigResponse {
//             owner: "owner".to_string(),
//             new_owner: None,
//             stake_token: helper.base.amp_token.get_address_string(),
//             epoch_period: 259200,
//             unbond_period: 1814400,
//             validators: vec![
//                 "val1".to_string(),
//                 "val2".to_string(),
//                 "val3".to_string(),
//                 "val4".to_string()
//             ],
//             fee_config: FeeConfig {
//                 protocol_fee_contract: Addr::unchecked("fee_new"),
//                 protocol_reward_fee: Decimal::from_ratio(10u128, 100u128)
//             },
//             delegation_strategy: eris::hub::DelegationStrategy::Uniform
//         }
//     );

//     Ok(())
// }

// #[test]
// fn happy_case() -> Result<()> {
//     let mut router = mock_app_validators(Some(100));
//     let router_ref = &mut router;
//     let helper = EscrowHelper::init(router_ref);

//     for i in 5..100 {
//         helper.hub_add_validator(router_ref, format!("val{0}", i))?;
//     }

//     helper.hub_bond(router_ref, "user1", 100_000000, "uluna")?;

//     helper.ve_lock_lp(router_ref, "user1", 1_000000, WEEK * 3)?;
//     helper.ve_lock_lp(router_ref, "user2", 1_000000, WEEK * 104)?;
//     helper.ve_lock_lp(router_ref, "user3", 1_000000, WEEK * 3)?;
//     helper.ve_lock_lp(router_ref, "user4", 10_000000, WEEK * 50)?;
//     helper.ve_lock_lp(router_ref, "user5", 10_000000, WEEK * 50)?;

//     helper.amp_vote(router_ref, "user1", vec![("val1".into(), 5000), ("val4".into(), 5000)])?;
//     helper.amp_vote(router_ref, "user2", vec![("val1".into(), 9000), ("val2".into(), 1000)])?;
//     helper.amp_vote(router_ref, "user3", vec![("val1".into(), 1000), ("val4".into(), 9000)])?;
//     helper.amp_vote(router_ref, "user4", vec![("val5".into(), 2400), ("val6".into(), 7600)])?;
//     helper.amp_vote(router_ref, "user5", vec![("val7".into(), 2500), ("val8".into(), 7500)])?;

//     router_ref.next_period(1);

//     helper.emp_add_points(
//         router_ref,
//         vec![
//             (
//                 "val1".into(),
//                 vec![
//                     EmpInfo {
//                         umerit_points: Uint128::new(10_000000),
//                         decaying_period: None,
//                     },
//                     EmpInfo {
//                         umerit_points: Uint128::new(1_000000),
//                         decaying_period: Some(12), // 3 months
//                     },
//                 ],
//             ),
//             point("val2", 20_000000, Some(12)),
//             point("val9", 1_000000, Some(3)),
//             point("val10", 20_000000, Some(3)),
//         ],
//     )?;

//     // admin at each first monday of month
//     helper.amp_tune(router_ref)?;
//     helper.hub_tune(router_ref)?;

//     let wanted = helper.hub_query_wanted_delegations(router_ref)?;

//     // val5, val7, val9 missing due to too few spaces (only 5 available).
//     assert_eq!(
//         wanted.delegations,
//         vec![
//             ("val2".into(), Uint128::new(22500152)),
//             ("val10".into(), Uint128::new(22019299)),
//             ("val6".into(), Uint128::new(19467469)),
//             ("val8".into(), Uint128::new(19211318)),
//             ("val1".into(), Uint128::new(16801760))
//         ]
//     );

//     let results = helper.hub_rebalance(router_ref)?;
//     results.assert_attribute("wasm-erishub/rebalanced", attr("uluna_moved", "83198238")).unwrap();

//     let delegations = helper.hub_query_all_delegations(router_ref)?;

//     let sum: u128 = delegations.iter().map(|d| d.amount.amount.u128()).sum();
//     assert_eq!(sum, 100_000000u128);

//     assert_eq!(
//         delegations
//             .into_iter()
//             .map(|d| (d.validator, d.amount.amount))
//             .sorted_by(|(_, a), (_, b)| b.cmp(a))
//             .collect::<Vec<_>>(),
//         vec![
//             ("val2".into(), Uint128::new(22500152)),
//             ("val10".into(), Uint128::new(22019299)),
//             ("val6".into(), Uint128::new(19467469)),
//             ("val8".into(), Uint128::new(19211318)),
//             ("val1".into(), Uint128::new(16801760 + 2))
//         ]
//     );

//     Ok(())
// }

// pub fn point(
//     val: impl Into<String>,
//     amount: u128,
//     decaying: Option<u64>,
// ) -> (String, Vec<EmpInfo>) {
//     (
//         val.into(),
//         vec![EmpInfo {
//             umerit_points: Uint128::new(amount),
//             decaying_period: decaying, // 3 months
//         }],
//     )
// }
