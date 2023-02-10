use std::vec;

use cosmwasm_std::attr;
use cosmwasm_std::testing::mock_info;

use eris::ampz::{
    ExecuteMsg, Execution, ExecutionDetail, ExecutionResponse, QueryMsg, Schedule, StateResponse,
};

use crate::contract::execute;
use crate::error::ContractError;
use crate::testing::helpers::{mock_env_at_timestamp, query_helper, query_helper_fail, setup_test};

//--------------------------------------------------------------------------------------------------
// Execution
//--------------------------------------------------------------------------------------------------

#[test]
fn proper_instantiation() {
    let deps = setup_test();

    let res: StateResponse = query_helper(deps.as_ref(), QueryMsg::State {});
    assert_eq!(
        res,
        StateResponse {
            next_id: 1
        },
    );
}

#[test]
fn setup_execution() {
    let mut deps = setup_test();
    let execution = Execution {
        destination: eris::ampz::DestinationState::DepositAmplifier {},
        schedule: Schedule {
            interval_s: 100,
            start: None,
        },
        user: "user".into(),
        source: eris::ampz::Source::Claim,
    };

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("other_user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: false,
            execution: execution.clone(),
        },
    )
    .unwrap_err();

    assert_eq!(res, ContractError::MustBeSameUser {});

    // add with valid user
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: false,
            execution: execution.clone(),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 0);
    assert_eq!(res.attributes, vec![attr("action", "ampz/add_execution"), attr("id", "1")]);

    let res = query_helper::<ExecutionResponse>(
        deps.as_ref(),
        QueryMsg::Execution {
            id: 1,
        },
    );
    assert_eq!(
        res.detail,
        ExecutionDetail {
            id: 1,
            execution: execution.clone(),
            last_execution: 1000 - 100,
            can_execute: true
        }
    );

    // add same again without override -> error
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(2000),
        mock_info("user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: true,
            execution: execution.clone(),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 0);
    assert_eq!(res.attributes, vec![attr("action", "ampz/add_execution"), attr("id", "2")]);

    // 1 does not exist anymore, as it was overwritten
    let err = query_helper_fail(
        deps.as_ref(),
        QueryMsg::Execution {
            id: 1,
        },
    );
    assert_eq!(err.to_string(), "eris::ampz::Execution not found".to_string());

    let res = query_helper::<ExecutionResponse>(
        deps.as_ref(),
        QueryMsg::Execution {
            id: 2,
        },
    );

    assert_eq!(
        res.detail,
        ExecutionDetail {
            id: 2,
            execution,
            last_execution: 2000 - 100,
            can_execute: true
        }
    );
}
