use crate::error::{ContractError, ContractResult};
use crate::extensions::executionext::ExecutionExt;
use crate::state::State;
use cosmwasm_std::{attr, Attribute, DepsMut, Env, MessageInfo, Response, Uint128};
use eris::ampz::Execution;
use eris::constants::HOUR;
use std::{cmp, vec};

pub fn add_execution(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    execution: Execution,
    overwrite: bool,
) -> ContractResult {
    if execution.user != info.sender {
        return Err(ContractError::MustBeSameUser {});
    }

    if execution.schedule.interval_s < 6 * HOUR {
        return Err(ContractError::IntervalTooShort {});
    }

    let state = State::default();

    execution.validate(&deps, &state)?;

    let source = execution.source.try_get_uniq_key();
    let new_id = state.id.load(deps.storage)?;

    if let Some(source) = source {
        // if the source has a unique id, each user can only create a single automation with this id.
        if overwrite {
            let result = state
                .execution_user_source
                .load(deps.storage, (execution.user.to_string(), source.clone()));

            if let Ok(old_id) = result {
                // remove existing execution
                state.executions.remove(deps.storage, old_id)?;
                state
                    .execution_user_source
                    .remove(deps.storage, (execution.user.clone(), source.clone()));
            }
        } else if state
            .execution_user_source
            .has(deps.storage, (execution.user.to_string(), source.clone()))
        {
            return Err(ContractError::ExecutionSourceCanOnlyBeUsedOnce {});
        }

        state.execution_user_source.save(
            deps.storage,
            (execution.user.clone(), source),
            &new_id,
        )?;
    }

    state.executions.save(deps.storage, new_id, &execution)?;

    // subbing the interval from the start allows the first execution to be on the start time.
    let initial_execution = cmp::max(
        execution.schedule.start.unwrap_or_else(|| env.block.time.seconds()),
        env.block.time.seconds(),
    )
    // can't go below epoch start
    .saturating_sub(execution.schedule.interval_s);

    state.last_execution.save(deps.storage, new_id, &initial_execution)?;

    state.id.save(deps.storage, &(new_id + 1))?;

    Ok(Response::new()
        .add_attribute("action", "ampz/add_execution")
        .add_attribute("id", new_id.to_string()))
}

pub fn remove_executions(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    ids: Option<Vec<Uint128>>,
) -> ContractResult {
    let state = State::default();

    let mut attrs: Vec<Attribute> = vec![];

    if let Some(ids) = ids {
        // if ids specified remove all ids
        for id in ids {
            let execution = state.get_by_id(deps.storage, id.u128())?;

            if execution.user != info.sender {
                return Err(ContractError::MustBeSameUser {});
            }

            state.executions.remove(deps.storage, id.u128())?;
            state.last_execution.remove(deps.storage, id.u128());

            let source = execution.source.try_get_uniq_key();
            if let Some(source) = source {
                state.execution_user_source.remove(deps.storage, (execution.user, source));
            }

            attrs.push(attr("removed_id", id.to_string()));
        }
    } else {
        // if nothing specified remove all from user
        let executions = state.get_by_user(deps.storage, info.sender.to_string())?;
        for execution in executions {
            state.executions.remove(deps.storage, execution.0)?;
            state.last_execution.remove(deps.storage, execution.0);

            let source = execution.1.source.try_get_uniq_key();
            if let Some(source) = source {
                state.execution_user_source.remove(deps.storage, (execution.1.user, source));
            }
            attrs.push(attr("removed_id", execution.0.to_string()));
        }
    }

    Ok(Response::new().add_attribute("action", "ampz/remove_executions").add_attributes(attrs))
}
