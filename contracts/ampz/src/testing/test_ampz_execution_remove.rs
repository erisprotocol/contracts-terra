use cosmwasm_std::{attr, testing::mock_info};

use crate::{contract::execute, error::ContractError};

use super::helpers::{add_default_execution, mock_env_at_timestamp, setup_test};

#[test]
fn check_execution_remove() {
    let mut deps = setup_test();

    let (id, _) = add_default_execution(&mut deps);
    add_default_execution(&mut deps);
    add_default_execution(&mut deps);

    execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("nobody", &[]),
        eris::ampz::ExecuteMsg::RemoveExecutions {
            ids: None,
        },
    )
    .unwrap();

    let result = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("attacker", &[]),
        eris::ampz::ExecuteMsg::RemoveExecutions {
            ids: Some(vec![id]),
        },
    )
    .unwrap_err();

    assert_eq!(result, ContractError::MustBeSameUser {});

    let result = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("user", &[]),
        eris::ampz::ExecuteMsg::RemoveExecutions {
            ids: Some(vec![id]),
        },
    )
    .unwrap();

    assert_eq!(
        result.attributes,
        vec![attr("action", "ampz/remove_executions"), attr("removed_id", "1")]
    );

    let result = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("user", &[]),
        eris::ampz::ExecuteMsg::RemoveExecutions {
            ids: None,
        },
    )
    .unwrap();

    assert_eq!(
        result.attributes,
        vec![
            attr("action", "ampz/remove_executions"),
            attr("removed_id", "2"),
            attr("removed_id", "3")
        ]
    );
}
