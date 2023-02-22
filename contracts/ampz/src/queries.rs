use cosmwasm_std::{Deps, Env, Order, StdResult};
use cw_storage_plus::Bound;
use itertools::Itertools;

use crate::state::State;
use eris::ampz::{
    AstroportConfig, ConfigResponse, ExecutionDetail, ExecutionResponse, ExecutionsResponse,
    FeeConfig, StateResponse, UserInfoResponse,
};

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub fn config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = State::default();

    Ok(ConfigResponse {
        owner: state.owner.load(deps.storage)?.into(),
        controller: state.controller.load(deps.storage)?.into(),
        new_owner: state.new_owner.may_load(deps.storage)?.map(|addr| addr.into()),
        hub: state.hub.load(deps.storage)?.0.into(),
        farms: state.farms.load(deps.storage)?.into_iter().map(|a| a.0.to_string()).collect_vec(),
        astroport: state.astroport.load(deps.storage).map(|a| AstroportConfig {
            generator: a.generator.0.to_string(),
            coins: a.coins,
        })?,
        zapper: state.zapper.load(deps.storage)?.0.into(),
        fee: state.fee.load(deps.storage).map(|f| FeeConfig {
            fee_bps: f.fee_bps,
            operator_bps: f.operator_bps,
            receiver: f.receiver.to_string(),
        })?,
    })
}

pub fn state(deps: Deps) -> StdResult<StateResponse> {
    let state = State::default();

    Ok(StateResponse {
        next_id: state.id.load(deps.storage)?,
    })
}

pub fn user_info(deps: Deps, env: Env, user: String) -> StdResult<UserInfoResponse> {
    let state = State::default();

    let executions = state
        .executions
        .idx
        .user
        .prefix(user)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (id, execution) = item?;
            let last_execution = state.last_execution.load(deps.storage, id)?;
            let can_execute =
                last_execution + execution.schedule.interval_s < env.block.time.seconds();

            Ok(ExecutionDetail {
                id,
                execution,
                last_execution,
                can_execute,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    Ok(UserInfoResponse {
        executions,
    })
}

pub fn execution(deps: Deps, env: Env, id: u128) -> StdResult<ExecutionResponse> {
    let state = State::default();
    let execution = state.executions.load(deps.storage, id)?;
    let last_execution = state.last_execution.load(deps.storage, id)?;
    let can_execute = last_execution + execution.schedule.interval_s < env.block.time.seconds();

    Ok(ExecutionResponse {
        detail: ExecutionDetail {
            id,
            execution,
            last_execution,
            can_execute,
        },
    })
}

pub fn executions(
    deps: Deps,
    start_after: Option<u128>,
    limit: Option<u32>,
) -> StdResult<ExecutionsResponse> {
    let state = State::default();

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let executions = state
        .executions
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    Ok(ExecutionsResponse {
        executions,
    })
}
