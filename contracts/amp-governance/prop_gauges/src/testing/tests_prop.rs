use std::vec;

use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage};
use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, Decimal, OwnedDeps, SubMsg, Uint128, VoteOption, WasmMsg,
};

use eris::governance_helper::{get_period, EPOCH_START, WEEK};
use eris::prop_gauges::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, PropDetailResponse, PropInfo, PropUserInfo,
    PropVotersResponse, PropsResponse, QueryMsg, UserPropResponseItem, UserVotesResponse,
};
use eris::voting_escrow::LockInfoResponse;
use itertools::Itertools;

use crate::contract::{execute, instantiate};
use crate::state::State;
use crate::testing::helpers::query_helper_env;

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
            owner: "owner".to_string(),
            escrow_addr: "escrow".to_string(),
            hub_addr: "hub".to_string(),
            quorum_bps: 500,
            use_weighted_vote: false,
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 0);

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
            owner: Addr::unchecked("owner"),
            escrow_addr: Addr::unchecked("escrow"),
            hub_addr: Addr::unchecked("hub"),
            quorum_bps: 500,
            use_weighted_vote: false
        }
    );

    let res: PropsResponse = query_helper(
        deps.as_ref(),
        QueryMsg::ActiveProps {
            limit: None,
            start_after: None,
        },
    );
    assert_eq!(
        res,
        PropsResponse {
            props: vec![]
        }
    );
}

#[test]
fn check_init_prop() {
    let mut deps = setup_test();

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("nobody", &[]),
        ExecuteMsg::InitProp {
            proposal_id: 1,
            end_time_s: EPOCH_START + WEEK * 10,
        },
    )
    .unwrap_err();

    assert_eq!(res.to_string(), "Generic error: unauthorized");

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::InitProp {
            proposal_id: 1,
            end_time_s: EPOCH_START - WEEK * 10,
        },
    )
    .unwrap_err();
    assert_eq!(res.to_string(), "Generic error: Invalid time");

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::InitProp {
            proposal_id: 1,
            end_time_s: EPOCH_START + WEEK * 10,
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 0);

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::InitProp {
            proposal_id: 1,
            end_time_s: EPOCH_START + WEEK * 10,
        },
    )
    .unwrap_err();
    assert_eq!(res.to_string(), "Generic error: prop 1 already initialized.");

    let res: PropsResponse = query_helper_env(
        deps.as_ref(),
        QueryMsg::ActiveProps {
            limit: None,
            start_after: None,
        },
        EPOCH_START + WEEK,
    );
    assert_eq!(
        res,
        PropsResponse {
            props: vec![(
                1,
                PropInfo {
                    abstain_vp: Uint128::zero(),
                    no_vp: Uint128::zero(),
                    nwv_vp: Uint128::zero(),
                    yes_vp: Uint128::zero(),
                    current_vote: None,
                    end_time_s: EPOCH_START + WEEK * 10,
                    period: get_period(EPOCH_START + WEEK * 10).unwrap(),
                    total_vp: Uint128::zero()
                }
            )]
        }
    );

    let res: PropsResponse = query_helper_env(
        deps.as_ref(),
        QueryMsg::ActiveProps {
            limit: None,
            start_after: None,
        },
        EPOCH_START + WEEK * 10 - 1,
    );
    assert_eq!(
        res,
        PropsResponse {
            props: vec![(
                1,
                PropInfo {
                    abstain_vp: Uint128::zero(),
                    no_vp: Uint128::zero(),
                    nwv_vp: Uint128::zero(),
                    yes_vp: Uint128::zero(),
                    current_vote: None,
                    end_time_s: EPOCH_START + WEEK * 10,
                    period: get_period(EPOCH_START + WEEK * 10).unwrap(),
                    total_vp: Uint128::zero()
                }
            )]
        }
    );

    let res: PropsResponse = query_helper_env(
        deps.as_ref(),
        QueryMsg::ActiveProps {
            limit: None,
            start_after: None,
        },
        EPOCH_START + WEEK * 11,
    );
    assert_eq!(
        res,
        PropsResponse {
            props: vec![]
        }
    );

    let _res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::InitProp {
            proposal_id: 2,
            end_time_s: EPOCH_START + WEEK * 11,
        },
    )
    .unwrap();

    let res: PropsResponse = query_helper_env(
        deps.as_ref(),
        QueryMsg::ActiveProps {
            limit: None,
            start_after: None,
        },
        EPOCH_START + WEEK * 10 - 1,
    );
    assert_eq!(
        res,
        PropsResponse {
            props: vec![
                (
                    1,
                    PropInfo {
                        abstain_vp: Uint128::zero(),
                        no_vp: Uint128::zero(),
                        nwv_vp: Uint128::zero(),
                        yes_vp: Uint128::zero(),
                        current_vote: None,
                        end_time_s: EPOCH_START + WEEK * 10,
                        period: get_period(EPOCH_START + WEEK * 10).unwrap(),
                        total_vp: Uint128::zero()
                    },
                ),
                (
                    2,
                    PropInfo {
                        abstain_vp: Uint128::zero(),
                        no_vp: Uint128::zero(),
                        nwv_vp: Uint128::zero(),
                        yes_vp: Uint128::zero(),
                        current_vote: None,
                        end_time_s: EPOCH_START + WEEK * 11,
                        period: get_period(EPOCH_START + WEEK * 11).unwrap(),
                        total_vp: Uint128::zero()
                    },
                )
            ]
        }
    );
}

#[test]
fn vote_prop() {
    let deps = setup_test();
    let mut deps = setup_props(deps);

    deps.querier.set_lock("user", 5, 5);
    deps.querier.set_lock("user2", 100, 100);

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(EPOCH_START),
        mock_info("user", &[]),
        ExecuteMsg::Vote {
            proposal_id: 3,
            vote: cosmwasm_std::VoteOption::Yes,
        },
    )
    .unwrap();

    // user does not reach quorum
    assert_eq!(res.messages.len(), 0);

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(EPOCH_START + WEEK),
        mock_info("user2", &[]),
        ExecuteMsg::Vote {
            proposal_id: 3,
            vote: cosmwasm_std::VoteOption::No,
        },
    )
    .unwrap();

    // user reaches quorum
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "hub".to_string(),
            msg: to_binary(&eris::hub::ExecuteMsg::Vote {
                proposal_id: 3,
                vote: cosmwasm_std::VoteOption::No
            })
            .unwrap(),
            funds: vec![]
        }))
    );

    let res: PropsResponse = query_helper_env(
        deps.as_ref(),
        QueryMsg::ActiveProps {
            limit: None,
            start_after: None,
        },
        EPOCH_START + WEEK * 2 + 1,
    );
    assert_eq!(
        res,
        PropsResponse {
            props: vec![(
                3,
                PropInfo {
                    abstain_vp: Uint128::zero(),
                    no_vp: Uint128::new(198),
                    nwv_vp: Uint128::zero(),
                    yes_vp: Uint128::new(7),
                    current_vote: Some(cosmwasm_std::VoteOption::No),
                    end_time_s: EPOCH_START + WEEK * 3,
                    period: get_period(EPOCH_START + WEEK * 3).unwrap(),
                    total_vp: Uint128::new(204)
                },
            )]
        }
    );

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(EPOCH_START + WEEK),
        mock_info("user2", &[]),
        ExecuteMsg::Vote {
            proposal_id: 3,
            vote: cosmwasm_std::VoteOption::Yes,
        },
    )
    .unwrap();

    // user changed it to yes
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "hub".to_string(),
            msg: to_binary(&eris::hub::ExecuteMsg::Vote {
                proposal_id: 3,
                vote: cosmwasm_std::VoteOption::Yes
            })
            .unwrap(),
            funds: vec![]
        }))
    );

    let res: PropsResponse = query_helper_env(
        deps.as_ref(),
        QueryMsg::ActiveProps {
            limit: None,
            start_after: None,
        },
        EPOCH_START + WEEK * 2 + 1,
    );
    assert_eq!(
        res,
        PropsResponse {
            props: vec![(
                3,
                PropInfo {
                    abstain_vp: Uint128::zero(),
                    no_vp: Uint128::zero(),
                    nwv_vp: Uint128::zero(),
                    yes_vp: Uint128::new(198 + 7),
                    current_vote: Some(cosmwasm_std::VoteOption::Yes),
                    end_time_s: EPOCH_START + WEEK * 3,
                    period: get_period(EPOCH_START + WEEK * 3).unwrap(),
                    total_vp: Uint128::new(204)
                },
            )]
        }
    );
}

#[test]
fn remove_user() {
    let deps = setup_test();
    let mut deps = setup_props(deps);

    deps.querier.set_lock("user", 5, 5);
    deps.querier.set_lock("user2", 100, 100);

    let _res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(EPOCH_START),
        mock_info("user", &[]),
        ExecuteMsg::Vote {
            proposal_id: 3,
            vote: cosmwasm_std::VoteOption::Yes,
        },
    )
    .unwrap();

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(EPOCH_START + WEEK),
        mock_info("user2", &[]),
        ExecuteMsg::Vote {
            proposal_id: 3,
            vote: cosmwasm_std::VoteOption::No,
        },
    )
    .unwrap();
    assert_eq!(res.messages.len(), 1);

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(EPOCH_START + WEEK),
        mock_info("user2", &[]),
        ExecuteMsg::Vote {
            proposal_id: 2,
            vote: cosmwasm_std::VoteOption::Yes,
        },
    )
    .unwrap();
    assert_eq!(res.messages.len(), 1);

    let prop: PropDetailResponse = query_helper(
        deps.as_ref(),
        QueryMsg::PropDetail {
            user: Some("user2".to_string()),
            proposal_id: 3,
        },
    );
    assert_eq!(
        prop,
        PropDetailResponse {
            user: Some(PropUserInfo {
                current_vote: cosmwasm_std::VoteOption::No,
                vp: Uint128::new(198),
                user: Addr::unchecked("user2")
            }),
            prop: PropInfo {
                abstain_vp: Uint128::zero(),
                no_vp: Uint128::new(198),
                nwv_vp: Uint128::zero(),
                yes_vp: Uint128::new(7),
                current_vote: Some(cosmwasm_std::VoteOption::No),
                end_time_s: EPOCH_START + WEEK * 3,
                period: get_period(EPOCH_START + WEEK * 3).unwrap(),
                total_vp: Uint128::new(204)
            }
        }
    );

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(EPOCH_START + WEEK),
        mock_info("attacker", &[]),
        ExecuteMsg::RemoveUser {
            user: "user2".to_string(),
        },
    )
    .unwrap_err();
    assert_eq!(res.to_string(), "Generic error: unauthorized");

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(EPOCH_START + WEEK),
        mock_info("owner", &[]),
        ExecuteMsg::RemoveUser {
            user: "user".to_string(),
        },
    )
    .unwrap();
    // nothing happens as user had not enough VP to make a change
    assert_eq!(res.messages.len(), 0);

    let prop: PropDetailResponse = query_helper(
        deps.as_ref(),
        QueryMsg::PropDetail {
            user: Some("user".to_string()),
            proposal_id: 3,
        },
    );
    assert_eq!(
        prop,
        PropDetailResponse {
            user: None,
            prop: PropInfo {
                abstain_vp: Uint128::zero(),
                no_vp: Uint128::new(198),
                nwv_vp: Uint128::zero(),
                yes_vp: Uint128::zero(),
                current_vote: Some(cosmwasm_std::VoteOption::No),
                end_time_s: EPOCH_START + WEEK * 3,
                period: get_period(EPOCH_START + WEEK * 3).unwrap(),
                total_vp: Uint128::new(204)
            }
        }
    );

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(EPOCH_START + WEEK),
        mock_info("owner", &[]),
        ExecuteMsg::RemoveUser {
            user: "user2".to_string(),
        },
    )
    .unwrap();
    // dropping below quorum (both votes) -> going for abstain
    assert_eq!(res.messages.len(), 2);
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "hub".to_string(),
            msg: to_binary(&eris::hub::ExecuteMsg::Vote {
                proposal_id: 2,
                vote: cosmwasm_std::VoteOption::Abstain
            })
            .unwrap(),
            funds: vec![]
        }))
    );
    assert_eq!(
        res.messages[1],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "hub".to_string(),
            msg: to_binary(&eris::hub::ExecuteMsg::Vote {
                proposal_id: 3,
                vote: cosmwasm_std::VoteOption::Abstain
            })
            .unwrap(),
            funds: vec![]
        }))
    );

    let prop: PropDetailResponse = query_helper(
        deps.as_ref(),
        QueryMsg::PropDetail {
            user: Some("user2".to_string()),
            proposal_id: 3,
        },
    );
    assert_eq!(
        prop,
        PropDetailResponse {
            user: None,
            prop: PropInfo {
                abstain_vp: Uint128::zero(),
                no_vp: Uint128::zero(),
                nwv_vp: Uint128::zero(),
                yes_vp: Uint128::zero(),
                current_vote: Some(cosmwasm_std::VoteOption::Abstain),
                end_time_s: EPOCH_START + WEEK * 3,
                period: get_period(EPOCH_START + WEEK * 3).unwrap(),
                total_vp: Uint128::new(204)
            }
        }
    );
}

#[test]
fn test_query_voters() {
    let deps = setup_test();
    let mut deps = setup_props(deps);

    for n in 1..101 {
        let user = format!("user{0}", n);
        deps.querier.set_lock(user.clone(), n, 5);

        let vote = if n % 2 == 0 {
            VoteOption::Yes
        } else {
            VoteOption::No
        };

        let _res = execute(
            deps.as_mut(),
            mock_env_at_timestamp(EPOCH_START),
            mock_info(user.as_str(), &[]),
            ExecuteMsg::Vote {
                proposal_id: 3,
                vote,
            },
        )
        .unwrap();
    }

    let res: PropVotersResponse = query_helper(
        deps.as_ref(),
        QueryMsg::PropVoters {
            proposal_id: 3,
            start_after: None,
            limit: None,
        },
    );

    assert_eq!(
        res,
        PropVotersResponse {
            voters: vec![
                (102, addr("user100"), VoteOption::Yes),
                (101, addr("user99"), VoteOption::No),
                (100, addr("user98"), VoteOption::Yes),
                (99, addr("user97"), VoteOption::No),
                (98, addr("user96"), VoteOption::Yes),
                (97, addr("user95"), VoteOption::No),
                (96, addr("user94"), VoteOption::Yes),
                (95, addr("user93"), VoteOption::No),
                (94, addr("user92"), VoteOption::Yes),
                (93, addr("user91"), VoteOption::No),
            ]
        }
    );

    let last = res.voters[res.voters.len() - 1].clone();

    let res: PropVotersResponse = query_helper(
        deps.as_ref(),
        QueryMsg::PropVoters {
            proposal_id: 3,
            start_after: Some((last.0, last.1.to_string())),
            limit: None,
        },
    );

    assert_eq!(
        res,
        PropVotersResponse {
            voters: vec![
                (92, addr("user90"), VoteOption::Yes),
                (91, addr("user89"), VoteOption::No),
                (90, addr("user88"), VoteOption::Yes),
                (89, addr("user87"), VoteOption::No),
                (88, addr("user86"), VoteOption::Yes),
                (87, addr("user85"), VoteOption::No),
                (86, addr("user84"), VoteOption::Yes),
                (85, addr("user83"), VoteOption::No),
                (84, addr("user82"), VoteOption::Yes),
                (83, addr("user81"), VoteOption::No)
            ]
        }
    );

    let res: PropVotersResponse = query_helper(
        deps.as_ref(),
        QueryMsg::PropVoters {
            proposal_id: 3,
            start_after: Some((50, "".to_string())),
            limit: Some(2),
        },
    );

    assert_eq!(
        res,
        PropVotersResponse {
            voters: vec![
                (49, addr("user47"), VoteOption::No),
                (48, addr("user46"), VoteOption::Yes),
            ]
        }
    );

    let res: PropVotersResponse = query_helper(
        deps.as_ref(),
        QueryMsg::PropVoters {
            proposal_id: 3,
            start_after: Some((50, "z".to_string())),
            limit: Some(2),
        },
    );

    assert_eq!(
        res,
        PropVotersResponse {
            voters: vec![
                (50, addr("user48"), VoteOption::Yes),
                (49, addr("user47"), VoteOption::No),
            ]
        }
    );
}

#[test]
fn test_query_user_votes() {
    let deps = setup_test();
    let mut deps = setup_props(deps);

    let user = format!("user{0}", 1);
    deps.querier.set_lock(user.clone(), 1, 5);

    let _res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(EPOCH_START),
        mock_info(user.as_str(), &[]),
        ExecuteMsg::Vote {
            proposal_id: 1,
            vote: VoteOption::Yes,
        },
    )
    .unwrap();
    let _res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(EPOCH_START),
        mock_info(user.as_str(), &[]),
        ExecuteMsg::Vote {
            proposal_id: 2,
            vote: VoteOption::Yes,
        },
    )
    .unwrap();

    let _res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(EPOCH_START),
        mock_info(user.as_str(), &[]),
        ExecuteMsg::Vote {
            proposal_id: 3,
            vote: VoteOption::No,
        },
    )
    .unwrap();

    let res: UserVotesResponse = query_helper(
        deps.as_ref(),
        QueryMsg::UserVotes {
            user: user.clone(),
            start_after: None,
            limit: None,
        },
    );

    assert_eq!(
        res,
        UserVotesResponse {
            props: vec![
                UserPropResponseItem {
                    id: 3,
                    current_vote: VoteOption::No,
                    vp: Uint128::new(3)
                },
                UserPropResponseItem {
                    id: 2,
                    current_vote: VoteOption::Yes,
                    vp: Uint128::new(4)
                },
                UserPropResponseItem {
                    id: 1,
                    current_vote: VoteOption::Yes,
                    vp: Uint128::new(5)
                },
            ]
        }
    );

    let res: UserVotesResponse = query_helper(
        deps.as_ref(),
        QueryMsg::UserVotes {
            user: user.clone(),
            start_after: Some(3),
            limit: Some(1),
        },
    );
    assert_eq!(
        res,
        UserVotesResponse {
            props: vec![UserPropResponseItem {
                id: 2,
                current_vote: VoteOption::Yes,
                vp: Uint128::new(4)
            },]
        }
    );

    let res: UserVotesResponse = query_helper(
        deps.as_ref(),
        QueryMsg::UserVotes {
            user,
            start_after: Some(2),
            limit: None,
        },
    );
    assert_eq!(
        res,
        UserVotesResponse {
            props: vec![UserPropResponseItem {
                id: 1,
                current_vote: VoteOption::Yes,
                vp: Uint128::new(5)
            },]
        }
    );
}

#[test]
fn test_query_active_props() {
    let mut deps = setup_test();

    for n in 1..101 {
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("owner", &[]),
            ExecuteMsg::InitProp {
                proposal_id: n,
                end_time_s: EPOCH_START + WEEK * n,
            },
        )
        .unwrap();
    }

    let res: PropsResponse = query_helper_env(
        deps.as_ref(),
        QueryMsg::ActiveProps {
            limit: Some(3),
            start_after: None,
        },
        EPOCH_START + WEEK * 10 + 1,
    );
    assert_eq!(res.props.iter().map(|p| p.0).collect_vec(), vec![11, 12, 13]);

    let res: PropsResponse = query_helper_env(
        deps.as_ref(),
        QueryMsg::ActiveProps {
            limit: Some(3),
            start_after: Some(res.props.last().unwrap().1.end_time_s),
        },
        EPOCH_START + WEEK * 10 + 1,
    );
    assert_eq!(res.props.into_iter().map(|p| p.0).collect_vec(), vec![14, 15, 16]);
}

#[test]
fn test_query_finished_props() {
    let mut deps = setup_test();

    for n in 1..101 {
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("owner", &[]),
            ExecuteMsg::InitProp {
                proposal_id: n,
                end_time_s: EPOCH_START + WEEK * n,
            },
        )
        .unwrap();
    }

    let res: PropsResponse = query_helper_env(
        deps.as_ref(),
        QueryMsg::FinishedProps {
            limit: Some(3),
            start_after: None,
        },
        EPOCH_START + WEEK * 10 + 1,
    );
    assert_eq!(res.props.iter().map(|p| p.0).collect_vec(), vec![10, 9, 8]);

    let res: PropsResponse = query_helper_env(
        deps.as_ref(),
        QueryMsg::FinishedProps {
            limit: Some(3),
            start_after: Some(res.props.last().unwrap().1.end_time_s),
        },
        EPOCH_START + WEEK * 10 + 1,
    );
    assert_eq!(res.props.into_iter().map(|p| p.0).collect_vec(), vec![7, 6, 5]);
}

fn addr(str: &str) -> Addr {
    Addr::unchecked(str)
}

#[test]
fn update_vote() {
    let deps = setup_test();
    let mut deps = setup_props(deps);

    deps.querier.set_lock("user", 5, 5);
    deps.querier.set_lock("user2", 100, 100);

    execute(
        deps.as_mut(),
        mock_env_at_timestamp(EPOCH_START),
        mock_info("user", &[]),
        ExecuteMsg::Vote {
            proposal_id: 3,
            vote: cosmwasm_std::VoteOption::Yes,
        },
    )
    .unwrap();

    deps.querier.set_lock("user", 10000, 10);
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(EPOCH_START),
        // coming from user -> unauthorized
        mock_info("user", &[]),
        ExecuteMsg::UpdateVote {
            user: "user".to_string(),
            lock_info: LockInfoResponse {
                amount: Uint128::zero(),
                coefficient: Decimal::zero(),
                start: 0,
                end: 10,
                slope: Uint128::new(1),
                fixed_amount: Uint128::new(10000),
                voting_power: Uint128::new(10),
            },
        },
    )
    .unwrap_err();
    assert_eq!(res.to_string(), "Unauthorized");

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(EPOCH_START),
        // coming from escrow
        mock_info("escrow", &[]),
        ExecuteMsg::UpdateVote {
            user: "user".to_string(),
            lock_info: LockInfoResponse {
                amount: Uint128::zero(),
                coefficient: Decimal::zero(),
                start: 0,
                end: 10,
                slope: Uint128::new(1),
                fixed_amount: Uint128::new(10000),
                voting_power: Uint128::new(10),
            },
        },
    )
    .unwrap();
    assert_eq!(res.messages.len(), 1);

    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "hub".to_string(),
            msg: to_binary(&eris::hub::ExecuteMsg::Vote {
                proposal_id: 3,
                vote: cosmwasm_std::VoteOption::Yes
            })
            .unwrap(),
            funds: vec![]
        }))
    );
}

#[test]
fn transferring_ownership() {
    let mut deps = setup_test();
    let state = State::default();

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake", &[]),
        ExecuteMsg::ProposeNewOwner {
            new_owner: "jake".to_string(),
            expires_in: 1000,
        },
    )
    .unwrap_err();

    assert_eq!(err.to_string(), "Generic error: Unauthorized");

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::ProposeNewOwner {
            new_owner: "jake".to_string(),
            expires_in: 1000,
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 0);

    let config = state.config.load(deps.as_ref().storage).unwrap();
    assert_eq!(config.owner, Addr::unchecked("owner"));

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("pumpkin", &[]),
        ExecuteMsg::ClaimOwnership {},
    )
    .unwrap_err();

    assert_eq!(err.to_string(), "Generic error: Unauthorized");

    let res =
        execute(deps.as_mut(), mock_env(), mock_info("jake", &[]), ExecuteMsg::ClaimOwnership {})
            .unwrap();

    assert_eq!(res.messages.len(), 0);

    let config = state.config.load(deps.as_ref().storage).unwrap();
    assert_eq!(config.owner, Addr::unchecked("jake"));
}

#[test]
fn update_config() {
    let mut deps = setup_test();

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake", &[]),
        ExecuteMsg::UpdateConfig {
            quorum_bps: Some(1000),
            use_weighted_vote: None,
        },
    )
    .unwrap_err();
    assert_eq!(res.to_string(), "Generic error: unauthorized");

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::UpdateConfig {
            quorum_bps: Some(12000),
            use_weighted_vote: None,
        },
    )
    .unwrap_err();
    assert_eq!(res.to_string(), "Generic error: Basic points conversion error. 12000 > 10000");

    execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::UpdateConfig {
            quorum_bps: Some(5000),
            use_weighted_vote: None,
        },
    )
    .unwrap();

    let res: ConfigResponse = query_helper(deps.as_ref(), QueryMsg::Config {});

    assert_eq!(
        res,
        ConfigResponse {
            owner: Addr::unchecked("owner"),
            escrow_addr: Addr::unchecked("escrow"),
            hub_addr: Addr::unchecked("hub"),
            quorum_bps: 5000,
            use_weighted_vote: false
        }
    );
}

fn setup_props(
    mut deps: OwnedDeps<MockStorage, MockApi, CustomQuerier>,
) -> OwnedDeps<MockStorage, MockApi, CustomQuerier> {
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::InitProp {
            proposal_id: 1,
            end_time_s: EPOCH_START + WEEK,
        },
    )
    .unwrap();

    execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::InitProp {
            proposal_id: 2,
            end_time_s: EPOCH_START + WEEK * 2,
        },
    )
    .unwrap();

    execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::InitProp {
            proposal_id: 3,
            end_time_s: EPOCH_START + WEEK * 3,
        },
    )
    .unwrap();

    deps
}
