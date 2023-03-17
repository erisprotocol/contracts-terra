use std::vec;

use astroport::asset::{native_asset, token_asset};
use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{attr, coins, Addr, Uint128};

use eris::ampz::{
    ExecuteMsg, Execution, ExecutionDetail, ExecutionResponse, QueryMsg, Schedule, StateResponse,
};
use eris::constants::{DAY, HOUR};

use crate::constants::CONTRACT_DENOM;
use crate::contract::execute;
use crate::error::ContractError;
use crate::state::State;
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
            next_id: Uint128::new(1)
        },
    );
}

#[test]
fn setup_execution() {
    let mut deps = setup_test();
    let execution = Execution {
        destination: eris::ampz::DestinationState::DepositAmplifier {},
        schedule: Schedule {
            interval_s: 6 * HOUR,
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
        mock_env_at_timestamp(DAY),
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
            id: Uint128::new(1),
        },
    );
    assert_eq!(
        res.detail,
        ExecutionDetail {
            id: Uint128::new(1),
            execution: execution.clone(),
            last_execution: DAY - 6 * HOUR,
            can_execute: true
        }
    );

    // add same again with override -> no error
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY * 2),
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
            id: Uint128::new(1),
        },
    );
    assert_eq!(err.to_string(), "eris::ampz::Execution not found".to_string());

    let res = query_helper::<ExecutionResponse>(
        deps.as_ref(),
        QueryMsg::Execution {
            id: Uint128::new(2),
        },
    );

    assert_eq!(
        res.detail,
        ExecutionDetail {
            id: Uint128::new(2),
            execution,
            last_execution: DAY * 2 - 6 * HOUR,
            can_execute: true
        }
    );
}

#[test]
fn setup_execution_farm() {
    let mut deps = setup_test();
    let mut execution = Execution {
        destination: eris::ampz::DestinationState::DepositFarm {
            farm: "unknown".into(),
        },
        schedule: Schedule {
            interval_s: 100,
            start: None,
        },
        user: "user".into(),
        source: eris::ampz::Source::Claim,
    };

    // add with invalid interval
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: false,
            execution: execution.clone(),
        },
    )
    .unwrap_err();

    assert_eq!(res, ContractError::IntervalTooShort {});

    execution = Execution {
        destination: eris::ampz::DestinationState::DepositFarm {
            farm: "unknown".into(),
        },
        schedule: Schedule {
            interval_s: HOUR * 6,
            start: None,
        },
        user: "user".into(),
        source: eris::ampz::Source::Claim,
    };

    // add with invalid farm
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: false,
            execution: execution.clone(),
        },
    )
    .unwrap_err();

    assert_eq!(res, ContractError::FarmNotSupported("unknown".into()));

    // add with valid farm
    execution.destination = eris::ampz::DestinationState::DepositFarm {
        farm: "farm1".into(),
    };
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
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
            id: Uint128::new(1),
        },
    );
    assert_eq!(
        res.detail,
        ExecutionDetail {
            id: Uint128::new(1),
            execution,
            last_execution: DAY - 6 * HOUR,
            can_execute: true
        }
    );
}

#[test]
fn test_deposit() {
    let mut deps = setup_test();

    let state = State::default();

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("user", &coins(10, CONTRACT_DENOM)),
        ExecuteMsg::Deposit {
            assets: vec![
                native_asset(CONTRACT_DENOM.into(), Uint128::new(10)),
                token_asset(Addr::unchecked("astro"), Uint128::new(50)),
            ],
        },
    )
    .unwrap_err();
    assert_eq!(res, ContractError::IsNotExecuting {});

    state.is_executing.save(deps.as_mut().storage, &true).unwrap();

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("user", &coins(10, CONTRACT_DENOM)),
        ExecuteMsg::Deposit {
            assets: vec![
                native_asset(CONTRACT_DENOM.into(), Uint128::new(10)),
                native_asset(CONTRACT_DENOM.into(), Uint128::new(10)),
                token_asset(Addr::unchecked(CONTRACT_DENOM), Uint128::new(50)),
            ],
        },
    )
    .unwrap_err();
    assert_eq!(res, ContractError::DuplicatedAsset {});

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("user", &coins(10, CONTRACT_DENOM)),
        ExecuteMsg::Deposit {
            assets: vec![
                native_asset(CONTRACT_DENOM.into(), Uint128::new(10)),
                token_asset(Addr::unchecked(CONTRACT_DENOM), Uint128::new(50)),
                token_asset(Addr::unchecked(CONTRACT_DENOM), Uint128::new(50)),
            ],
        },
    )
    .unwrap_err();
    assert_eq!(res, ContractError::DuplicatedAsset {});

    // duplicated asset for token + native should not throw
    execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("user", &coins(10, CONTRACT_DENOM)),
        ExecuteMsg::Deposit {
            assets: vec![
                native_asset(CONTRACT_DENOM.into(), Uint128::new(10)),
                token_asset(Addr::unchecked(CONTRACT_DENOM), Uint128::new(50)),
            ],
        },
    )
    .unwrap_err();
    // we are also not allowing native assets that have the same denom as token assets
    assert_eq!(res, ContractError::DuplicatedAsset {});

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("user", &coins(12, CONTRACT_DENOM)),
        ExecuteMsg::Deposit {
            assets: vec![
                native_asset(CONTRACT_DENOM.into(), Uint128::new(10)),
                token_asset(Addr::unchecked("astro"), Uint128::new(50)),
            ],
        },
    )
    .unwrap_err();

    assert_eq!(
        res.to_string(),
        "Generic error: Native token balance mismatch between the argument and the transferred"
            .to_string()
    );

    execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("user", &coins(10, CONTRACT_DENOM)),
        ExecuteMsg::Deposit {
            assets: vec![
                native_asset(CONTRACT_DENOM.into(), Uint128::new(10)),
                token_asset(Addr::unchecked("astro"), Uint128::new(50)),
            ],
        },
    )
    .unwrap();
}
