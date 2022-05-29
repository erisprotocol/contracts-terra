use cosmwasm_std::{Addr, Decimal, Deps, Env, Order, StdResult, Uint128};
use cw_storage_plus::Bound;

use eris_staking::hub::{
    Batch, ConfigResponse, PendingBatch, StateResponse, UnbondRequestsByBatchResponseItem,
    UnbondRequestsByUserResponseItem,
};

use crate::helpers::{query_cw20_total_supply, query_delegations};
use crate::state::State;

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub fn config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = State::default();

    Ok(ConfigResponse {
        owner: state.owner.load(deps.storage)?.into(),
        new_owner: state.new_owner.may_load(deps.storage)?.map(|addr| addr.into()),
        stake_token: state.stake_token.load(deps.storage)?.into(),
        epoch_period: state.epoch_period.load(deps.storage)?,
        unbond_period: state.unbond_period.load(deps.storage)?,
        validators: state.validators.load(deps.storage)?,
        fee_config: state.fee_config.load(deps.storage)?,
    })
}

pub fn state(deps: Deps, env: Env) -> StdResult<StateResponse> {
    let state = State::default();

    let stake_token = state.stake_token.load(deps.storage)?;
    let total_ustake = query_cw20_total_supply(&deps.querier, &stake_token)?;

    let validators = state.validators.load(deps.storage)?;
    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address)?;
    let total_uluna: u128 = delegations.iter().map(|d| d.amount).sum();

    let exchange_rate = if total_ustake.is_zero() {
        Decimal::one()
    } else {
        Decimal::from_ratio(total_uluna, total_ustake)
    };

    Ok(StateResponse {
        total_ustake,
        total_uluna: Uint128::new(total_uluna),
        exchange_rate,
        unlocked_coins: state.unlocked_coins.load(deps.storage)?,
    })
}

pub fn pending_batch(deps: Deps) -> StdResult<PendingBatch> {
    let state = State::default();
    state.pending_batch.load(deps.storage)
}

pub fn previous_batch(deps: Deps, id: u64) -> StdResult<Batch> {
    let state = State::default();
    state.previous_batches.load(deps.storage, id)
}

pub fn previous_batches(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<Batch>> {
    let state = State::default();

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    state
        .previous_batches
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, v) = item?;
            Ok(v)
        })
        .collect()
}

pub fn unbond_requests_by_batch(
    deps: Deps,
    id: u64,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<UnbondRequestsByBatchResponseItem>> {
    let state = State::default();

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let mut start: Option<Bound<&Addr>> = None;
    let addr: Addr;
    if let Some(start_after) = start_after {
        if let Ok(start_after_addr) = deps.api.addr_validate(&start_after) {
            addr = start_after_addr;
            start = Some(Bound::exclusive(&addr));
        }
    }

    state
        .unbond_requests
        .prefix(id)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, v) = item?;
            Ok(v.into())
        })
        .collect()
}

pub fn unbond_requests_by_user(
    deps: Deps,
    user: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<UnbondRequestsByUserResponseItem>> {
    let state = State::default();

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let addr = deps.api.addr_validate(&user)?;
    let start = start_after.map(|id| Bound::exclusive((id, &addr)));

    state
        .unbond_requests
        .idx
        .user
        .prefix(user)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, v) = item?;

            Ok(v.into())
        })
        .collect()
}
