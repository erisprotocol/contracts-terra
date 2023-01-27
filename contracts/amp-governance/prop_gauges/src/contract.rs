use astroport::asset::addr_validate_to_lower;
use astroport::common::{claim_ownership, drop_ownership_proposal, propose_new_owner};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128,
    VoteOption,
};
use cw2::{get_contract_version, set_contract_version};
use eris::CustomResponse;

use eris::governance_helper::get_period;
use eris::helpers::bps::BasicPoints;
use eris::prop_gauges::{ExecuteMsg, InstantiateMsg, MigrateMsg, PropInfo, PropUserInfo, QueryMsg};
use eris::voting_escrow::{get_lock_info, get_total_voting_power_at_by_period, LockInfoResponse};

use crate::error::ContractError;
use crate::queries::{
    get_active_props, get_finished_props, get_prop_detail, get_prop_voters, get_user_votes,
};
use crate::state::{Config, State};
use crate::vote::{get_vote_msg, remove_vote_of_user, update_vote_state};

/// Contract name that is used for migration.
const CONTRACT_NAME: &str = "prop-gauges";
/// Contract version that is used for migration.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// const DAY: u64 = 86400;
/// It is possible to tune pools once every 14 days
// const TUNE_COOLDOWN: u64 = WEEK * 3;

type ExecuteResult = Result<Response, ContractError>;

/// Creates a new contract with the specified parameters in the [`InstantiateMsg`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ExecuteResult {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let state = State::default();

    BasicPoints::try_from(msg.quorum_bps)?;

    state.config.save(
        deps.storage,
        &Config {
            owner: addr_validate_to_lower(deps.api, &msg.owner)?,
            escrow_addr: addr_validate_to_lower(deps.api, &msg.escrow_addr)?,
            hub_addr: addr_validate_to_lower(deps.api, &msg.hub_addr)?,
            quorum_bps: msg.quorum_bps,
            use_weighted_vote: msg.use_weighted_vote,
        },
    )?;

    Ok(Response::default())
}

/// Exposes all the execute functions available in the contract.
///
/// ## Execute messages
/// * **ExecuteMsg::Vote { votes }** Casts votes for pools
///
/// * **ExecuteMsg::TunePools** Launches pool tuning
///
/// * **ExecuteMsg::ChangePoolsLimit { limit }** Changes the number of pools which are eligible
/// to receive allocation points
///
/// * **ExecuteMsg::UpdateConfig { blacklisted_voters_limit }** Changes the number of blacklisted
/// voters that can be kicked at once
///
/// * **ExecuteMsg::ProposeNewOwner { owner, expires_in }** Creates a new request to change
/// contract ownership.
///
/// * **ExecuteMsg::DropOwnershipProposal {}** Removes a request to change contract ownership.
///
/// * **ExecuteMsg::ClaimOwnership {}** Claims contract ownership.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> ExecuteResult {
    match msg {
        ExecuteMsg::InitProp {
            proposal_id,
            end_time_s,
        } => init_prop(deps, env, info, proposal_id, end_time_s),
        ExecuteMsg::Vote {
            proposal_id,
            vote,
        } => handle_vote(deps, env, info, proposal_id, vote),
        ExecuteMsg::UpdateVote {
            user,
            lock_info,
        } => update_vote(deps, env, info, user, lock_info),
        ExecuteMsg::RemoveUser {
            user,
        } => remove_user(deps, env, info, user),
        ExecuteMsg::RemoveProp {
            proposal_id,
        } => remove_prop(deps, env, info, proposal_id),
        ExecuteMsg::UpdateConfig {
            quorum_bps,
            use_weighted_vote,
        } => update_config(deps, info, quorum_bps, use_weighted_vote),
        ExecuteMsg::ProposeNewOwner {
            new_owner,
            expires_in,
        } => {
            let state = State::default();
            let config: Config = state.config.load(deps.storage)?;

            propose_new_owner(
                deps,
                info,
                env,
                new_owner,
                expires_in,
                config.owner,
                state.ownership_proposal,
            )
            .map_err(Into::into)
        },
        ExecuteMsg::DropOwnershipProposal {} => {
            let state = State::default();
            let config: Config = state.config.load(deps.storage)?;

            drop_ownership_proposal(deps, info, config.owner, state.ownership_proposal)
                .map_err(Into::into)
        },
        ExecuteMsg::ClaimOwnership {} => {
            let state = State::default();
            claim_ownership(deps, info, env, state.ownership_proposal, |deps, new_owner| {
                let state = State::default();
                state
                    .config
                    .update::<_, StdError>(deps.storage, |mut v| {
                        v.owner = new_owner;
                        Ok(v)
                    })
                    .map(|_| ())
            })
            .map_err(Into::into)
        },
    }
}

fn init_prop(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
    end_time_s: u64,
) -> ExecuteResult {
    let state = State::default();
    let config = state.config.load(deps.storage)?;
    config.assert_owner(&info.sender)?;

    let prop = state.props.may_load(deps.storage, proposal_id)?;

    if prop.is_some() {
        return Err(
            StdError::generic_err(format!("prop {0} already initialized.", proposal_id)).into()
        );
    }

    if end_time_s < env.block.time.seconds() {
        return Err(StdError::generic_err("End time can't be in the past.").into());
    }

    let period = get_period(end_time_s)?;

    state.props.save(
        deps.storage,
        proposal_id,
        &PropInfo {
            end_time_s,
            period,
            total_vp: get_total_voting_power_at_by_period(
                &deps.querier,
                config.escrow_addr,
                period,
            )?,
            current_vote: None,
            no_vp: Uint128::zero(),
            abstain_vp: Uint128::zero(),
            yes_vp: Uint128::zero(),
            nwv_vp: Uint128::zero(),
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "prop/init_prop")
        .add_attribute("prop", proposal_id.to_string())
        .add_attribute("end", period.to_string()))
}

/// The function checks that:
/// * the user voting power is > 0,
/// * all pool addresses are valid LP token addresses,
/// * 'votes' vector doesn't contain duplicated pool addresses,
/// * sum of all BPS values <= 10000.
///
/// The function cancels changes applied by previous votes and apply new votes for the next period.
/// New vote parameters are saved in [`USER_INFO`].
///
/// The function returns [`Response`] in case of success or [`ContractError`] in case of errors.
///
/// * **votes** is a vector of pairs ([`String`], [`u16`]).
/// Tuple consists of pool address and percentage of user's voting power for a given pool.
/// Percentage should be in BPS form.
fn handle_vote(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
    vote: VoteOption,
) -> ExecuteResult {
    let sender = info.sender;

    let state = State::default();
    let config = state.config.load(deps.storage)?;

    let ve_lock_info = get_lock_info(&deps.querier, &config.escrow_addr, &sender)?;
    let vamp = ve_lock_info.voting_power + ve_lock_info.fixed_amount;
    if vamp.is_zero() {
        return Err(ContractError::ZeroVotingPower {});
    }

    let user_info =
        state.get_user_info(deps.storage, proposal_id, &sender)?.unwrap_or(PropUserInfo {
            user: sender.clone(),
            current_vote: VoteOption::Abstain,
            vp: Uint128::zero(),
        });

    let (user, vote_msg) = update_vote_state(
        &env,
        &deps.querier,
        deps.storage,
        &state,
        &config,
        &sender,
        proposal_id,
        None,
        vote,
        user_info,
        &ve_lock_info,
    )?;

    Ok(Response::new()
        .add_optional_message(vote_msg)
        .add_attribute("action", "prop/vote")
        .add_attribute("vp", user.vp))
}

fn update_vote(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
    ve_lock_info: LockInfoResponse,
) -> ExecuteResult {
    let sender = deps.api.addr_validate(&user)?;
    let state = State::default();
    let config = state.config.load(deps.storage)?;

    if info.sender != config.escrow_addr {
        return Err(ContractError::Unauthorized {});
    }

    let mut response = Response::new();

    for (proposal_id, prop) in state.all_active_props(deps.storage, &env)?.into_iter() {
        let user = state.get_user_info(deps.storage, proposal_id, &sender)?;

        if let Some(user_info) = user {
            let (_, vote_msg) = update_vote_state(
                &env,
                &deps.querier,
                deps.storage,
                &state,
                &config,
                &sender,
                proposal_id,
                Some(prop),
                user_info.current_vote.clone(),
                user_info,
                &ve_lock_info,
            )?;

            response = response
                .add_optional_message(vote_msg)
                .add_attribute("prop", proposal_id.to_string());
        }
    }

    Ok(response.add_attribute("action", "prop/update_vote"))
}

fn remove_user(deps: DepsMut, env: Env, info: MessageInfo, user: String) -> ExecuteResult {
    let state = State::default();
    let config = state.config.load(deps.storage)?;
    config.assert_owner(&info.sender)?;

    let user_addr = addr_validate_to_lower(deps.api, user)?;

    let mut response = Response::new();
    for (proposal_id, prop) in state.all_active_props(deps.storage, &env)?.into_iter() {
        let user = state.get_user_info(deps.storage, proposal_id, &user_addr)?;

        if let Some(user) = user {
            let mut prop = remove_vote_of_user(prop, &user)?;

            let (vote_msg, total_vp) =
                get_vote_msg(&deps.querier, &config, &mut prop, proposal_id)?;

            prop.total_vp = total_vp;

            state.props.save(deps.storage, proposal_id, &prop)?;
            state.users.remove(deps.storage, (proposal_id, user_addr.clone()))?;
            state.voters.remove(deps.storage, (proposal_id, user.vp.u128(), user_addr.clone()));

            response = response
                .add_optional_message(vote_msg)
                .add_attribute("removed-id", proposal_id.to_string());
        }
    }

    Ok(response.add_attribute("action", "prop/remove_user"))
}

fn remove_prop(deps: DepsMut, _env: Env, info: MessageInfo, proposal_id: u64) -> ExecuteResult {
    let state = State::default();
    let config = state.config.load(deps.storage)?;
    config.assert_owner(&info.sender)?;

    state.props.remove(deps.storage, proposal_id)?;

    Ok(Response::new().add_attribute("action", "prop/remove_prop"))
}

/// Only contract owner can call this function.  
/// The function sets a new limit of blacklisted voters that can be kicked at once.
///
/// * **blacklisted_voters_limit** is a new limit of blacklisted voters which can be kicked at once
///
/// * **main_pool** is a main pool address
///
/// * **main_pool_min_alloc** is a minimum percentage of ASTRO emissions that this pool should get every block
///
/// * **remove_main_pool** should the main pool be removed or not
fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    quorum_bps: Option<u16>,
    use_weighted_vote: Option<bool>,
) -> ExecuteResult {
    let state = State::default();
    let mut config = state.config.load(deps.storage)?;
    config.assert_owner(&info.sender)?;

    if let Some(quorum_bps) = quorum_bps {
        BasicPoints::try_from(quorum_bps)?;
        config.quorum_bps = quorum_bps;
    }

    if let Some(use_weighted_vote) = use_weighted_vote {
        config.use_weighted_vote = use_weighted_vote;
    }

    state.config.save(deps.storage, &config)?;

    Ok(Response::default().add_attribute("action", "prop/update_config"))
}

/// Expose available contract queries.
///
/// ## Queries
/// * **QueryMsg::UserInfo { user }** Fetch user information
///
/// * **QueryMsg::TuneInfo** Fetch last tuning information
///
/// * **QueryMsg::Config** Fetch contract config
///
/// * **QueryMsg::PoolInfo { pool_addr }** Fetch pool's voting information at the current period.
///
/// * **QueryMsg::PoolInfoAtPeriod { pool_addr, period }** Fetch pool's voting information at a specified period.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&State::default().config.load(deps.storage)?),
        QueryMsg::ActiveProps {
            start_after,
            limit,
        } => to_binary(&get_active_props(deps, env, start_after, limit)?),
        QueryMsg::FinishedProps {
            start_after,
            limit,
        } => to_binary(&get_finished_props(deps, env, start_after, limit)?),
        QueryMsg::PropDetail {
            user,
            proposal_id,
        } => to_binary(&get_prop_detail(deps, env, user, proposal_id)?),
        QueryMsg::PropVoters {
            proposal_id,
            start_after,
            limit,
        } => to_binary(&get_prop_voters(deps, env, proposal_id, start_after, limit)?),
        QueryMsg::UserVotes {
            user,
            limit,
            start_after,
        } => to_binary(&get_user_votes(deps, env, user, start_after, limit)?),
    }
}

/// Manages contract migration
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let contract_version = get_contract_version(deps.storage)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if contract_version.contract != CONTRACT_NAME {
        return Err(StdError::generic_err(format!(
            "contract_name does not match: prev: {0}, new: {1}",
            contract_version.contract, CONTRACT_VERSION
        ))
        .into());
    }

    Ok(Response::new()
        .add_attribute("previous_contract_name", &contract_version.contract)
        .add_attribute("previous_contract_version", &contract_version.version)
        .add_attribute("new_contract_name", CONTRACT_NAME)
        .add_attribute("new_contract_version", CONTRACT_VERSION))
}
