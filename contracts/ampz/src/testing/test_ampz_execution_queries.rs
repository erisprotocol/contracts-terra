use cosmwasm_std::testing::mock_info;
use eris::{
    ampz::{
        ExecuteMsg, Execution, ExecutionDetail, ExecutionsResponse, Schedule, UserInfoResponse,
    },
    constants::{DAY, HOUR},
};

use crate::contract::execute;

use super::helpers::{
    add_default_execution, mock_env_at_timestamp, query_helper, query_helper_time, setup_test,
};

#[test]
fn check_query_user_info() {
    let mut deps = setup_test();

    let execution1 = Execution {
        destination: eris::ampz::DestinationState::DepositAmplifier {},
        schedule: Schedule {
            interval_s: 8 * HOUR,
            start: None,
        },
        user: "user".into(),
        source: eris::ampz::Source::Claim,
    };

    execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: false,
            execution: execution1.clone(),
        },
    )
    .unwrap();

    let execution2 = Execution {
        destination: eris::ampz::DestinationState::DepositAmplifier {},
        schedule: Schedule {
            interval_s: 10 * HOUR,
            start: Some(2 * DAY),
        },
        user: "user".into(),
        source: eris::ampz::Source::AstroRewards {
            lps: vec!["lp1".into()],
        },
    };

    execute(
        deps.as_mut(),
        mock_env_at_timestamp(1005),
        mock_info("user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: false,
            execution: execution2.clone(),
        },
    )
    .unwrap();

    let (_, execution3) = add_default_execution(&mut deps);
    let (_, execution4) = add_default_execution(&mut deps);

    let res: UserInfoResponse = query_helper_time(
        deps.as_ref(),
        eris::ampz::QueryMsg::UserInfo {
            user: "user".into(),
        },
        DAY + 1,
    );
    assert_eq!(
        res,
        UserInfoResponse {
            executions: vec![
                ExecutionDetail {
                    id: 1,
                    execution: execution1.clone(),
                    last_execution: DAY - 8 * HOUR,
                    can_execute: true
                },
                ExecutionDetail {
                    id: 2,
                    execution: execution2.clone(),
                    last_execution: 2 * DAY - 10 * HOUR,
                    can_execute: false
                },
                ExecutionDetail {
                    id: 3,
                    execution: execution3.clone(),
                    last_execution: DAY - 6 * HOUR,
                    can_execute: true
                },
                ExecutionDetail {
                    id: 4,
                    execution: execution4.clone(),
                    last_execution: DAY - 6 * HOUR,
                    can_execute: true
                },
            ]
        }
    );

    let res: UserInfoResponse = query_helper_time(
        deps.as_ref(),
        eris::ampz::QueryMsg::UserInfo {
            user: "user".into(),
        },
        2 * DAY + 1,
    );
    assert_eq!(
        res,
        UserInfoResponse {
            executions: vec![
                ExecutionDetail {
                    id: 1,
                    execution: execution1,
                    last_execution: DAY - 8 * HOUR,
                    can_execute: true
                },
                ExecutionDetail {
                    id: 2,
                    execution: execution2,
                    last_execution: 2 * DAY - 10 * HOUR,
                    can_execute: true
                },
                ExecutionDetail {
                    id: 3,
                    execution: execution3,
                    last_execution: DAY - 6 * HOUR,
                    can_execute: true
                },
                ExecutionDetail {
                    id: 4,
                    execution: execution4,
                    last_execution: DAY - 6 * HOUR,
                    can_execute: true
                },
            ]
        }
    )
}

#[test]
fn check_query_executions() {
    let mut deps = setup_test();

    let executions = get_executions(&deps, None, None);
    assert_eq!(executions.executions, vec![]);

    let execution1 = Execution {
        destination: eris::ampz::DestinationState::DepositAmplifier {},
        schedule: Schedule {
            interval_s: 6 * HOUR,
            start: None,
        },
        user: "user".into(),
        source: eris::ampz::Source::Claim,
    };
    execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: false,
            execution: execution1.clone(),
        },
    )
    .unwrap();

    let executions = get_executions(&deps, None, None);
    assert_eq!(executions.executions, vec![(1, execution1.clone())]);

    let execution2 = Execution {
        destination: eris::ampz::DestinationState::DepositAmplifier {},
        schedule: Schedule {
            interval_s: 6 * HOUR,
            start: Some(2 * DAY),
        },
        user: "other_user".into(),
        source: eris::ampz::Source::AstroRewards {
            lps: vec!["lp1".into()],
        },
    };
    execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("other_user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: false,
            execution: execution2.clone(),
        },
    )
    .unwrap();

    let executions = get_executions(&deps, None, None);
    assert_eq!(executions.executions, vec![(1, execution1), (2, execution2.clone())]);

    let (_, execution3) = add_default_execution(&mut deps);
    let (_, execution4) = add_default_execution(&mut deps);

    let executions = get_executions(&deps, Some(1), None);
    assert_eq!(
        executions.executions,
        vec![(2, execution2.clone()), (3, execution3), (4, execution4.clone())]
    );

    // remove 1,3 replace 2 with 5
    execute(
        deps.as_mut(),
        mock_env_at_timestamp(1005),
        mock_info("user", &[]),
        ExecuteMsg::RemoveExecutions {
            ids: Some(vec![1, 3]),
        },
    )
    .unwrap();
    execute(
        deps.as_mut(),
        mock_env_at_timestamp(1005),
        mock_info("other_user", &[]),
        ExecuteMsg::RemoveExecutions {
            ids: None,
        },
    )
    .unwrap();
    execute(
        deps.as_mut(),
        mock_env_at_timestamp(1005),
        mock_info("other_user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: true,
            execution: execution2.clone(),
        },
    )
    .unwrap();

    let executions = get_executions(&deps, Some(1), None);
    assert_eq!(executions.executions, vec![(4, execution4), (5, execution2)]);
}

fn get_executions(
    deps: &cosmwasm_std::OwnedDeps<
        cosmwasm_std::MemoryStorage,
        cosmwasm_std::testing::MockApi,
        super::custom_querier::CustomQuerier,
    >,
    start_after: Option<u128>,
    limit: Option<u32>,
) -> ExecutionsResponse {
    let executions: ExecutionsResponse = query_helper(
        deps.as_ref(),
        eris::ampz::QueryMsg::Executions {
            start_after,
            limit,
        },
    );
    executions
}
