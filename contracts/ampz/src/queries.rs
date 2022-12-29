use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;
use itertools::Itertools;

use crate::state::State;
use eris::ampz::{
    AstroportConfig, ConfigResponse, ExecutionsResponse, StateResponse, UserInfoResponse,
};

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub fn config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = State::default();

    Ok(ConfigResponse {
        owner: state.owner.load(deps.storage)?.into(),
        executor: state.controller.load(deps.storage)?.into(),
        new_owner: state.new_owner.may_load(deps.storage)?.map(|addr| addr.into()),
        hub: state.hub.load(deps.storage)?.0.into(),
        farms: state.farms.load(deps.storage)?.into_iter().map(|a| a.0.to_string()).collect_vec(),
        astroport: state.astroport.load(deps.storage).map(|a| AstroportConfig {
            generator: a.generator.0.to_string(),
            coins: a.coins,
        })?,
        zapper: state.zapper.load(deps.storage)?.0.into(),
    })
}

pub fn state(deps: Deps) -> StdResult<StateResponse> {
    let state = State::default();

    Ok(StateResponse {
        id: state.id.load(deps.storage)?,
    })
}

pub fn user_info(deps: Deps, user: String) -> StdResult<UserInfoResponse> {
    let state = State::default();

    let executions = state
        .executions
        .idx
        .user
        .prefix(user)
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    Ok(UserInfoResponse {
        executions,
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
