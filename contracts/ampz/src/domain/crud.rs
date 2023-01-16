use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdError, StdResult};

use crate::state::State;
use eris::adapters::farm::Farm;
use eris::ampz::{Destination, Execution};

pub fn add_execution(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    execution: Execution,
    overwrite: bool,
) -> StdResult<Response> {
    if execution.user != info.sender {
        return Err(StdError::generic_err("can only be added by same user"));
    }

    let state = State::default();
    let source = execution.source.try_get_uniq_key();
    let new_id = state.id.load(deps.storage)?;

    if let Some(source) = source {
        // if the source has a unique id, each user can only create a single automation with this id.
        if overwrite {
            let result = state
                .execution_user_source
                .load(deps.storage, (execution.user.to_string(), source.clone()));

            if let Ok(old_id) = result {
                state.executions.remove(deps.storage, old_id)?;
                state
                    .execution_user_source
                    .remove(deps.storage, (execution.user.clone(), source.clone()));
            }
        } else if state
            .execution_user_source
            .has(deps.storage, (execution.user.to_string(), source.clone()))
        {
            return Err(StdError::generic_err("source already defined for the user"));
        }

        state.execution_user_source.save(
            deps.storage,
            (execution.user.clone(), source),
            &new_id,
        )?;
    }

    // match &execution.source {
    //     eris::ampz::Source::Claim => (),
    //     eris::ampz::Source::AstroRewards {
    //         ..
    //     } => (),
    //     eris::ampz::Source::Wallet {
    //         over,
    //         ..
    //     } => {

    //         if let AssetInfo::NativeToken { denom } = over.info {
    //             if denom == CONTRACT_DENOM {
    //                 if over.amount < Uint128::new(1e6) {
    //                     return Err(StdError::generic_err(format!("for the gas token threshold must be higher than 1_000_000", over.info)));
    //                 }
    //             }
    //         }

    //         let astro = state.astroport.load(deps.storage)?;

    //         if !astro.coins.contains(&over.info) {
    //             return Err(StdError::generic_err(format!("token {} not supported", over.info)));
    //         }
    //     },
    // }

    match &execution.destination {
        Destination::DepositAmplifier {} => (),
        Destination::DepositFarm {
            farm,
        } => {
            let allowed_farms = state.farms.load(deps.storage)?;
            let farm = Farm(deps.api.addr_validate(farm)?);
            if !allowed_farms.contains(&farm) {
                return Err(StdError::generic_err(format!("farm {} does not exist", farm.0)));
            }
        },
    }

    state.executions.save(deps.storage, new_id, &execution)?;

    let initial_execution = execution
        .schedule
        .start
        .unwrap_or_else(|| env.block.time.seconds())
        .checked_sub(execution.schedule.interval_s)
        .unwrap_or_default();

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
    ids: Option<Vec<u128>>,
) -> StdResult<Response> {
    let state = State::default();

    if let Some(ids) = ids {
        // if ids specified remove all ids
        for id in ids {
            let execution = state.get_by_id(deps.storage, id)?;

            if execution.user != info.sender {
                return Err(StdError::generic_err("can only be removed by creator"));
            }

            state.executions.remove(deps.storage, id)?;
            state.last_execution.remove(deps.storage, id);

            let source = execution.source.try_get_uniq_key();
            if let Some(source) = source {
                state.execution_user_source.remove(deps.storage, (execution.user, source));
            }
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
        }
    }

    Ok(Response::new().add_attribute("action", "ampz/remove_executions"))
}
