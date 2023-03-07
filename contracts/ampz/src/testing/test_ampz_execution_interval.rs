use std::vec;

use astroport::asset::native_asset;
use cosmwasm_std::testing::{mock_info, MockApi, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{coins, Addr, OwnedDeps, Uint128};

use eris::ampz::{CallbackMsg, ExecuteMsg, Execution, Schedule};
use eris::constants::{DAY, HOUR};
use protobuf::SpecialFields;

use crate::constants::CONTRACT_DENOM;
use crate::contract::execute;
use crate::error::ContractError;
use crate::protos::authz::MsgExec;
use crate::protos::proto::MsgWithdrawDelegatorReward;
use crate::testing::helpers::{mock_env_at_timestamp, setup_test};

use super::custom_querier::CustomQuerier;

#[test]
fn check_execution_interval() {
    let mut deps = setup_test();

    deps.querier.bank_querier.update_balance("user", coins(50, CONTRACT_DENOM));

    let interval_s = 6 * HOUR;
    let execution = Execution {
        destination: eris::ampz::DestinationState::DepositAmplifier {},
        schedule: Schedule {
            interval_s,
            start: None,
        },
        user: "user".into(),
        source: eris::ampz::Source::Claim,
    };

    let finish_execution = CallbackMsg::FinishExecution {
        destination: eris::ampz::DestinationRuntime::DepositAmplifier {},
        executor: Addr::unchecked("controller"),
    };

    controller_executes(&mut deps, execution, &finish_execution);

    // user is allowed to execute again anytime
    user_can_manually_execute_any_time(&mut deps, finish_execution);

    // cannot execute the same element with the same timestamp
    nobody_can_execute_before_interval(&mut deps);

    // anyone can execute again after the interval
    anyone_can_execute_after_interval(deps, interval_s);
}

fn controller_executes(
    deps: &mut OwnedDeps<cosmwasm_std::MemoryStorage, MockApi, CustomQuerier>,
    execution: Execution,
    finish_execution: &CallbackMsg,
) {
    execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: false,
            execution,
        },
    )
    .unwrap();

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("controller", &[]),
        ExecuteMsg::Execute {
            id: 1,
        },
    )
    .unwrap();

    // claim + deposit + finish
    assert_eq!(res.messages.len(), 3);
    assert_eq!(
        res.messages[0].msg,
        MsgExec {
            grantee: MOCK_CONTRACT_ADDR.to_string(),
            msgs: vec![MsgWithdrawDelegatorReward {
                delegator_address: "user".to_string(),
                validator_address: "val1".to_string(),
                special_fields: SpecialFields::default()
            }
            .to_any()
            .unwrap()],
            special_fields: SpecialFields::default()
        }
        .to_authz_cosmos_msg()
    );
    assert_eq!(
        res.messages[1].msg,
        CallbackMsg::AuthzDeposit {
            user_balance_start: vec![native_asset(CONTRACT_DENOM.to_string(), Uint128::new(50))],
            max_amount: None
        }
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &Addr::unchecked("user"))
        .unwrap()
    );
    assert_eq!(
        res.messages[2].msg,
        finish_execution
            .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &Addr::unchecked("user"))
            .unwrap()
    );

    // need to execute finish callback to allow next execution
    finish(deps, finish_execution.clone());
}

fn anyone_can_execute_after_interval(
    mut deps: OwnedDeps<cosmwasm_std::MemoryStorage, MockApi, CustomQuerier>,
    interval_s: u64,
) {
    execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000 + interval_s),
        mock_info("anyone", &[]),
        ExecuteMsg::Execute {
            id: 1,
        },
    )
    .unwrap();
}

fn nobody_can_execute_before_interval(
    deps: &mut OwnedDeps<cosmwasm_std::MemoryStorage, MockApi, CustomQuerier>,
) {
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("nobody", &[]),
        ExecuteMsg::Execute {
            id: 1,
        },
    )
    .unwrap_err();

    assert_eq!(res, ContractError::ExecutionInFuture(1000 + HOUR * 6));

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000 + HOUR * 6 - 1),
        mock_info("controller", &[]),
        ExecuteMsg::Execute {
            id: 1,
        },
    )
    .unwrap_err();

    assert_eq!(res, ContractError::ExecutionInFuture(1000 + HOUR * 6));
}

fn user_can_manually_execute_any_time(
    deps: &mut OwnedDeps<cosmwasm_std::MemoryStorage, MockApi, CustomQuerier>,
    finish_execution: CallbackMsg,
) {
    execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("user", &[]),
        ExecuteMsg::Execute {
            id: 1,
        },
    )
    .unwrap();
    finish(deps, finish_execution);
}

fn finish(
    deps: &mut OwnedDeps<cosmwasm_std::MemoryStorage, MockApi, CustomQuerier>,
    finish_execution: CallbackMsg,
) {
    deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, coins(100, CONTRACT_DENOM));
    execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::Callback(finish_execution.into_callback_wrapper(1, &Addr::unchecked("user"))),
    )
    .unwrap();
}
