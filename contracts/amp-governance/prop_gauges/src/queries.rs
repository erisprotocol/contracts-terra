use std::cmp;

use cosmwasm_std::{Addr, Deps, Env, StdResult, VoteOption};
use cw_storage_plus::Bound;
use eris::{
    prop_gauges::{
        PropDetailResponse, PropInfo, PropVotersResponse, PropsResponse, UserPropResponseItem,
        UserVotesResponse,
    },
    voting_escrow::{DEFAULT_LIMIT, MAX_LIMIT},
};

use crate::state::State;

pub fn get_active_props(
    deps: Deps,
    env: Env,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<PropsResponse> {
    let state = State::default();
    let current_time = env.block.time.seconds();
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let min = if let Some(start_after) = start_after {
        if start_after > current_time {
            // +1 is used, as the second parameter is 0 and pagination is correct see test_query_finished_props
            Some(Bound::inclusive((start_after + 1, 0)))
        } else {
            Some(Bound::inclusive((current_time, 0)))
        }
    } else {
        Some(Bound::inclusive((current_time, 0)))
    };

    let props = state
        .props
        .idx
        .time
        .range(deps.storage, min, None, cosmwasm_std::Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<(u64, PropInfo)>>>()?;

    Ok(PropsResponse {
        props,
    })
}

pub fn get_user_votes(
    deps: Deps,
    _env: Env,
    user: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<UserVotesResponse> {
    let state = State::default();
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let addr = deps.api.addr_validate(&user)?;

    let max = start_after.map(|start_after| Bound::exclusive((start_after, Addr::unchecked("a"))));

    let props = state
        .users
        .idx
        .user
        .prefix(addr)
        .range(deps.storage, None, max, cosmwasm_std::Order::Descending)
        .take(limit)
        .map(|item| -> StdResult<UserPropResponseItem> {
            let (id, prop) = item?;

            Ok(UserPropResponseItem {
                id: id.0,
                current_vote: prop.current_vote,
                vp: prop.vp,
            })
        })
        .collect::<StdResult<Vec<UserPropResponseItem>>>()?;

    Ok(UserVotesResponse {
        props,
    })
}

pub fn get_finished_props(
    deps: Deps,
    env: Env,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<PropsResponse> {
    let state = State::default();
    let current_time = env.block.time.seconds();
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let max = if let Some(start_after) = start_after {
        Some(Bound::exclusive((cmp::min(start_after, current_time), 0)))
    } else {
        Some(Bound::exclusive((current_time, 0)))
    };

    let props = state
        .props
        .idx
        .time
        .range(deps.storage, None, max, cosmwasm_std::Order::Descending)
        .take(limit)
        .collect::<StdResult<Vec<(u64, PropInfo)>>>()?;

    Ok(PropsResponse {
        props,
    })
}

pub fn get_prop_detail(
    deps: Deps,
    _env: Env,
    user: Option<String>,
    proposal_id: u64,
) -> StdResult<PropDetailResponse> {
    let state = State::default();

    let prop = state.props.load(deps.storage, proposal_id)?;

    let user = if let Some(user) = user {
        let addr = deps.api.addr_validate(&user)?;
        state.get_user_info(deps.storage, proposal_id, &addr)?
    } else {
        None
    };

    Ok(PropDetailResponse {
        prop,
        user,
    })
}

pub fn get_prop_voters(
    deps: Deps,
    _env: Env,
    proposal_id: u64,
    start_after: Option<(u128, String)>,
    limit: Option<u32>,
) -> StdResult<PropVotersResponse> {
    let state = State::default();

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = if let Some(start_after) = start_after {
        // we are not validating the addr here, as we want to allow searching without address.
        // (100, "") -> returns the first user with VP < 100.
        // (100, "z") -> returns the first user with VP <= 100
        let addr = Addr::unchecked(start_after.1);
        Some(Bound::exclusive((start_after.0, addr)))
    } else {
        None
    };

    let voters = state
        .voters
        .sub_prefix(proposal_id)
        .range(deps.storage, None, start, cosmwasm_std::Order::Descending)
        .take(limit)
        .map(|item| {
            let (key, vote) = item?;
            Ok((key.0, key.1, vote))
        })
        .collect::<StdResult<Vec<(u128, Addr, VoteOption)>>>()?;

    Ok(PropVotersResponse {
        voters,
    })
}
