use cosmwasm_std::{
    Addr, CosmosMsg, Env, QuerierWrapper, StdError, StdResult, Storage, Uint128, VoteOption,
};
use eris::{
    adapters::hub::Hub,
    governance_helper::{calc_voting_power, get_period},
    prop_gauges::{ConfigResponse, PropInfo, PropUserInfo},
    voting_escrow::{get_total_voting_power_at_by_period, LockInfoResponse},
};

use crate::{error::ContractError, state::State};

pub fn remove_vote_of_user(mut prop: PropInfo, user: &PropUserInfo) -> StdResult<PropInfo> {
    let vp = user.vp;

    if vp.is_zero() {
        return Ok(prop);
    }

    match user.current_vote {
        cosmwasm_std::VoteOption::Yes => prop.yes_vp = prop.yes_vp.checked_sub(vp)?,
        cosmwasm_std::VoteOption::No => prop.no_vp = prop.no_vp.checked_sub(vp)?,
        cosmwasm_std::VoteOption::Abstain => prop.abstain_vp = prop.abstain_vp.checked_sub(vp)?,
        cosmwasm_std::VoteOption::NoWithVeto => prop.nwv_vp = prop.nwv_vp.checked_sub(vp)?,
    };

    Ok(prop)
}

pub fn apply_vote_of_user(
    env: &Env,
    mut prop: PropInfo,
    ve_lock_info: &LockInfoResponse,
    vote: VoteOption,
    user: Addr,
) -> StdResult<(PropInfo, PropUserInfo)> {
    let current_period = get_period(env.block.time.seconds())?;
    let vp = calc_voting_power_for_prop(current_period, ve_lock_info, &prop);

    if vp.is_zero() {
        return Ok((
            prop,
            PropUserInfo {
                current_vote: VoteOption::Abstain,
                vp,
                user,
            },
        ));
    }

    match vote {
        cosmwasm_std::VoteOption::Yes => prop.yes_vp = prop.yes_vp.checked_add(vp)?,
        cosmwasm_std::VoteOption::No => prop.no_vp = prop.no_vp.checked_add(vp)?,
        cosmwasm_std::VoteOption::Abstain => prop.abstain_vp = prop.abstain_vp.checked_add(vp)?,
        cosmwasm_std::VoteOption::NoWithVeto => prop.nwv_vp = prop.nwv_vp.checked_add(vp)?,
    };

    Ok((
        prop,
        PropUserInfo {
            current_vote: vote,
            vp,
            user,
        },
    ))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn update_vote_state(
    env: &Env,
    querier: &QuerierWrapper,
    store: &mut dyn Storage,
    state: &State,
    config: &ConfigResponse,
    sender: &Addr,
    proposal_id: u64,
    prop: Option<PropInfo>,
    vote: VoteOption,
    user_info: PropUserInfo,
    ve_lock_info: &LockInfoResponse,
) -> Result<(PropUserInfo, Option<CosmosMsg>), ContractError> {
    let prop = if let Some(prop) = prop {
        prop
    } else {
        state.props.load(store, proposal_id).map_err(|_| {
            StdError::generic_err(format!("proposal with id {0} not initialized", proposal_id))
        })?
    };

    let prop = remove_vote_of_user(prop, &user_info)?;
    let (mut prop, user) = apply_vote_of_user(env, prop, ve_lock_info, vote, sender.clone())?;

    let (vote_msg, total_vp) = get_vote_msg(querier, config, &mut prop, proposal_id)?;
    prop.total_vp = total_vp;

    state.props.save(store, proposal_id, &prop)?;
    state.users.save(store, (proposal_id, sender.clone()), &user)?;
    state.voters.remove(store, (proposal_id, user_info.vp.u128(), sender.clone()));
    state.voters.save(store, (proposal_id, user.vp.u128(), sender.clone()), &user.current_vote)?;
    Ok((user, vote_msg))
}

pub fn get_vote_msg(
    querier: &QuerierWrapper,
    config: &ConfigResponse,
    prop: &mut PropInfo,
    proposal_id: u64,
) -> Result<(Option<CosmosMsg>, Uint128), ContractError> {
    let total_vp =
        get_total_voting_power_at_by_period(querier, config.escrow_addr.clone(), prop.period)?;

    if config.use_weighted_vote {
        let vote_msg: Option<CosmosMsg> = if prop.reached_quorum(total_vp, config.quorum_bps)? {
            let votes = prop.get_weighted_votes();
            Some(Hub(config.hub_addr.clone()).vote_weighted_msg(proposal_id, votes)?)
        } else {
            None
        };

        prop.current_vote = None;
        Ok((vote_msg, total_vp))
    } else {
        // if normal vote, check if the current vote is already set.
        let current_vote = prop.current_vote.clone();
        let mut wanted = prop.get_wanted_vote(total_vp, config.quorum_bps)?;
        let vote_msg: Option<CosmosMsg> = if wanted != current_vote {
            if let Some(wanted) = &wanted {
                Some(Hub(config.hub_addr.clone()).vote_msg(proposal_id, wanted.clone())?)
            } else if current_vote.is_some()
                && wanted.is_none()
                && current_vote != Some(VoteOption::Abstain)
            {
                // if we already voted and go back to "not voting", vote abstain instead.
                wanted = Some(VoteOption::Abstain);
                Some(Hub(config.hub_addr.clone()).vote_msg(proposal_id, VoteOption::Abstain)?)
            } else {
                None
            }
        } else {
            None
        };
        prop.current_vote = wanted;
        Ok((vote_msg, total_vp))
    }
}

fn calc_voting_power_for_prop(
    current_period: u64,
    ve_lock_info: &LockInfoResponse,
    prop: &PropInfo,
) -> Uint128 {
    // println!("{:?} {:?}", ve_lock_info, prop);
    let period = prop.period;
    let start = current_period;

    if start == period {
        ve_lock_info.voting_power + ve_lock_info.fixed_amount
    } else if ve_lock_info.end <= period {
        // the current period is after the voting end -> get default end power.
        ve_lock_info.fixed_amount
    } else {
        // The point before the intended period was found, thus we can calculate the user's voting power for the period we want
        calc_voting_power(ve_lock_info.slope, ve_lock_info.voting_power, start, period)
            + ve_lock_info.fixed_amount
    }
}
