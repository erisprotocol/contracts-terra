use std::collections::HashSet;
use std::convert::TryInto;

use astroport::asset::addr_validate_to_lower;
use astroport::common::{claim_ownership, drop_ownership_proposal, propose_new_owner};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdError, StdResult,
    Storage, Uint128,
};
use cw2::set_contract_version;
use eris::hub::get_hub_validators;
use itertools::Itertools;

use eris::amp_gauges::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, UserInfoResponse};
use eris::governance_helper::{calc_voting_power, get_period};
use eris::helpers::bps::BasicPoints;
use eris::voting_escrow::{get_lock_info, get_voting_power, LockInfoResponse, LockInfoVPResponse};

use crate::error::ContractError;
use crate::state::{
    Config, TuneInfo, UserInfo, VotedValidatorInfo, CONFIG, OWNERSHIP_PROPOSAL, TUNE_INFO,
    USER_INFO, VALIDATORS,
};
use crate::utils::{
    cancel_user_changes, filter_pools, get_validator_info, update_validator_info,
    validate_validators_limit, vote_for_pool,
};

/// Contract name that is used for migration.
const CONTRACT_NAME: &str = "amp-gauges";
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
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ExecuteResult {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    CONFIG.save(
        deps.storage,
        &Config {
            owner: addr_validate_to_lower(deps.api, &msg.owner)?,
            escrow_addr: addr_validate_to_lower(deps.api, &msg.escrow_addr)?,
            hub_addr: addr_validate_to_lower(deps.api, &msg.hub_addr)?,
            emp_registry_addr: addr_validate_to_lower(deps.api, &msg.emp_registry_addr)?,
            validators_limit: validate_validators_limit(msg.validators_limit)?,
        },
    )?;

    // Set tune_ts just for safety so the first tuning could happen in 2 weeks
    TUNE_INFO.save(
        deps.storage,
        &TuneInfo {
            tune_ts: env.block.time.seconds(),
            vamp_points: vec![],
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
        ExecuteMsg::Vote {
            votes,
        } => handle_vote(deps, env, info, votes),
        ExecuteMsg::UpdateVote {
            user,
            lock_info,
        } => update_vote(deps, env, info, user, lock_info),
        ExecuteMsg::TuneVamp {} => tune_vamp(deps, env, info),
        ExecuteMsg::UpdateConfig {
            validators_limit,
        } => update_config(deps, info, validators_limit),
        ExecuteMsg::ProposeNewOwner {
            new_owner,
            expires_in,
        } => {
            let config: Config = CONFIG.load(deps.storage)?;

            propose_new_owner(
                deps,
                info,
                env,
                new_owner,
                expires_in,
                config.owner,
                OWNERSHIP_PROPOSAL,
            )
            .map_err(Into::into)
        },
        ExecuteMsg::DropOwnershipProposal {} => {
            let config: Config = CONFIG.load(deps.storage)?;

            drop_ownership_proposal(deps, info, config.owner, OWNERSHIP_PROPOSAL)
                .map_err(Into::into)
        },
        ExecuteMsg::ClaimOwnership {} => {
            claim_ownership(deps, info, env, OWNERSHIP_PROPOSAL, |deps, new_owner| {
                CONFIG
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

// /// This function removes all votes applied by blacklisted voters.
// ///
// /// * **holders** list with blacklisted holders whose votes will be removed.
// fn kick_blacklisted_voters(deps: DepsMut, env: Env, voters: Vec<String>) -> ExecuteResult {
//     let block_period = get_period(env.block.time.seconds())?;
//     let config = CONFIG.load(deps.storage)?;

//     if voters.len() > config.blacklisted_voters_limit.unwrap_or(VOTERS_MAX_LIMIT) as usize {
//         return Err(ContractError::KickVotersLimitExceeded {});
//     }

//     // Check duplicated voters
//     let addrs_set = voters.iter().collect::<HashSet<_>>();
//     if voters.len() != addrs_set.len() {
//         return Err(ContractError::DuplicatedVoters {});
//     }

//     // Check if voters are blacklisted
//     let res: BlacklistedVotersResponse = deps.querier.query_wasm_smart(
//         config.escrow_addr,
//         &CheckVotersAreBlacklisted {
//             voters: voters.clone(),
//         },
//     )?;

//     if !res.eq(&BlacklistedVotersResponse::VotersBlacklisted {}) {
//         return Err(ContractError::Std(StdError::generic_err(res.to_string())));
//     }

//     for voter in voters {
//         let voter_addr = addr_validate_to_lower(deps.api, &voter)?;
//         if let Some(user_info) = USER_INFO.may_load(deps.storage, &voter_addr)? {
//             if user_info.lock_end > block_period {
//                 let user_last_vote_period = get_period(user_info.vote_ts)?;
//                 // Calculate voting power before changes
//                 let old_vp_at_period = calc_voting_power(
//                     user_info.slope,
//                     user_info.voting_power,
//                     user_last_vote_period,
//                     block_period,
//                 );

//                 // Cancel changes applied by previous votes
//                 user_info.votes.iter().try_for_each(|(pool_addr, bps)| {
//                     cancel_user_changes(
//                         deps.storage,
//                         block_period + 1,
//                         pool_addr,
//                         *bps,
//                         old_vp_at_period,
//                         user_info.slope,
//                         user_info.lock_end,
//                     )
//                 })?;

//                 let user_info = UserInfo {
//                     vote_ts: env.block.time.seconds(),
//                     lock_end: block_period,
//                     ..Default::default()
//                 };

//                 USER_INFO.save(deps.storage, &voter_addr, &user_info)?;
//             }
//         }
//     }

//     Ok(Response::new().add_attribute("action", "kick_holders"))
// }

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
    votes: Vec<(String, u16)>,
) -> ExecuteResult {
    let user = info.sender;
    let block_period = get_period(env.block.time.seconds())?;
    let config = CONFIG.load(deps.storage)?;
    let user_vp = get_voting_power(&deps.querier, &config.escrow_addr, &user)?;

    if user_vp.is_zero() {
        return Err(ContractError::ZeroVotingPower {});
    }

    let user_info = USER_INFO.may_load(deps.storage, &user)?.unwrap_or_default();

    // Check duplicated votes
    let addrs_set = votes.iter().cloned().map(|(addr, _)| addr).collect::<HashSet<_>>();
    if votes.len() != addrs_set.len() {
        return Err(ContractError::DuplicatedPools {});
    }

    let allowed_validators = get_hub_validators(&deps.querier, config.hub_addr)?;

    // Validating addrs and bps
    let votes = votes
        .into_iter()
        .map(|(addr, bps)| {
            if !allowed_validators.contains(&addr) {
                return Err(ContractError::InvalidValidatorAddress(addr));
            }
            let addr = addr_validate_to_lower(deps.api, addr)?;
            let bps: BasicPoints = bps.try_into()?;
            Ok((addr, bps))
        })
        .collect::<Result<Vec<_>, ContractError>>()?;

    // Check the bps sum is within the limit
    votes.iter().try_fold(BasicPoints::default(), |acc, (_, bps)| acc.checked_add(*bps))?;

    remove_votes_of_user(&user_info, block_period, deps.storage)?;

    let ve_lock_info = get_lock_info(&deps.querier, &config.escrow_addr, &user)?;
    let vp = ve_lock_info.amount;
    let coefficient = ve_lock_info.coefficient;

    apply_votest_of_user(votes, deps, block_period, user_vp, ve_lock_info, env, user)?;

    Ok(Response::new().add_attribute("action", "vote").add_attribute("vAMP", vp * coefficient))
}

fn apply_votest_of_user(
    votes: Vec<(Addr, BasicPoints)>,
    deps: DepsMut,
    block_period: u64,
    user_vp: Uint128,
    ve_lock_info: LockInfoResponse,
    env: Env,
    user: Addr,
) -> Result<(), ContractError> {
    votes.iter().try_for_each(|(validator_addr, bps)| {
        vote_for_pool(
            deps.storage,
            block_period + 1,
            validator_addr,
            *bps,
            user_vp,
            ve_lock_info.slope,
            ve_lock_info.end,
        )
    })?;
    let user_info = UserInfo {
        vote_ts: env.block.time.seconds(),
        voting_power: user_vp,
        slope: ve_lock_info.slope,
        lock_end: ve_lock_info.end,
        votes,
    };
    USER_INFO.save(deps.storage, &user, &user_info)?;
    Ok(())
}

fn remove_votes_of_user(
    user_info: &UserInfo,
    block_period: u64,
    storage: &mut dyn Storage,
) -> Result<(), ContractError> {
    if user_info.lock_end > block_period {
        let user_last_vote_period = get_period(user_info.vote_ts).unwrap_or(block_period);
        // Calculate voting power before changes
        let old_vp_at_period = calc_voting_power(
            user_info.slope,
            user_info.voting_power,
            user_last_vote_period,
            block_period,
        );

        // Cancel changes applied by previous votes
        user_info.votes.iter().try_for_each(|(pool_addr, bps)| {
            cancel_user_changes(
                storage,
                block_period + 1,
                pool_addr,
                *bps,
                old_vp_at_period,
                user_info.slope,
                user_info.lock_end,
            )
        })?;
    };
    Ok(())
}

fn update_vote(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
    lock: LockInfoVPResponse,
) -> ExecuteResult {
    let block_period = get_period(env.block.time.seconds())?;
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.escrow_addr {
        return Err(ContractError::Unauthorized {});
    }

    let user = addr_validate_to_lower(deps.api, user)?;
    let user_info = USER_INFO.may_load(deps.storage, &user)?;

    if let Some(user_info) = user_info {
        remove_votes_of_user(&user_info, block_period, deps.storage)?;

        if lock.voting_power.is_zero() {
            return Ok(Response::new().add_attribute("action", "update_vote_removed"));
        }

        apply_votest_of_user(
            user_info.votes,
            deps,
            block_period,
            lock.voting_power,
            lock.lock,
            env,
            user,
        )?;

        return Ok(Response::new()
            .add_attribute("action", "update_vote_changed")
            .add_attribute("vAMP", lock.voting_power));
    }

    Ok(Response::new().add_attribute("action", "update_vote_noop"))
}

/// The function checks that the last pools tuning happened >= 14 days ago.
/// Then it calculates voting power for each pool at the current period, filters all pools which
/// are not eligible to receive allocation points,
/// takes top X pools by voting power, where X is 'config.pools_limit', calculates allocation points
/// for these pools and applies allocation points in generator contract.
fn tune_vamp(deps: DepsMut, env: Env, info: MessageInfo) -> ExecuteResult {
    let config = CONFIG.load(deps.storage)?;
    config.assert_owner(&info.sender)?;

    let mut tune_info = TUNE_INFO.load(deps.storage)?;
    let block_period = get_period(env.block.time.seconds())?;

    let validator_votes: Vec<_> = VALIDATORS
        .keys(deps.as_ref().storage, None, None, Order::Ascending)
        .collect::<Vec<_>>()
        .into_iter()
        .map(|validator_addr| {
            let validator_addr = validator_addr?;

            let validator_info =
                update_validator_info(deps.storage, block_period, &validator_addr, None)?;

            println!("{:?}", validator_info);

            // Remove pools with zero voting power so we won't iterate over them in future
            if validator_info.vamp_amount.is_zero() {
                VALIDATORS.remove(deps.storage, &validator_addr)
            }
            Ok((validator_addr, validator_info.vamp_amount))
        })
        .collect::<StdResult<Vec<_>>>()?
        .into_iter()
        .filter(|(_, vamp_amount)| !vamp_amount.is_zero())
        .sorted_by(|(_, a), (_, b)| b.cmp(a)) // Sort in descending order
        .collect();

    println!("{:?}", validator_votes);

    tune_info.vamp_points = filter_pools(
        &deps.querier,
        &config.hub_addr,
        validator_votes,
        config.validators_limit, // +1 additional pool if we will need to remove the main pool
    )?;

    if tune_info.vamp_points.is_empty() {
        return Err(ContractError::TuneNoValidators {});
    }

    tune_info.tune_ts = env.block.time.seconds();
    TUNE_INFO.save(deps.storage, &tune_info)?;

    Ok(Response::new().add_attribute("action", "tune_vamp"))
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
fn update_config(deps: DepsMut, info: MessageInfo, validators_limit: Option<u64>) -> ExecuteResult {
    let mut config = CONFIG.load(deps.storage)?;

    config.assert_owner(&info.sender)?;

    if let Some(validators_limit) = validators_limit {
        config.validators_limit = validators_limit;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_attribute("action", "update_config"))
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
        QueryMsg::UserInfo {
            user,
        } => to_binary(&user_info(deps, user)?),
        QueryMsg::TuneInfo {} => to_binary(&TUNE_INFO.load(deps.storage)?),
        QueryMsg::Config {} => to_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::ValidatorInfo {
            validator_addr,
        } => to_binary(&validator_info(deps, env, validator_addr, None)?),
        QueryMsg::ValidatorInfoAtPeriod {
            validator_addr,
            period,
        } => to_binary(&validator_info(deps, env, validator_addr, Some(period))?),
    }
}

/// Returns user information.
fn user_info(deps: Deps, user: String) -> StdResult<UserInfoResponse> {
    let user_addr = addr_validate_to_lower(deps.api, &user)?;
    USER_INFO
        .may_load(deps.storage, &user_addr)?
        .map(UserInfo::into_response)
        .ok_or_else(|| StdError::generic_err("User not found"))
}

/// Returns pool's voting information at a specified period.
fn validator_info(
    deps: Deps,
    env: Env,
    validator_addr: String,
    period: Option<u64>,
) -> StdResult<VotedValidatorInfo> {
    let pool_addr = addr_validate_to_lower(deps.api, &validator_addr)?;
    let block_period = get_period(env.block.time.seconds())?;
    let period = period.unwrap_or(block_period);
    get_validator_info(deps.storage, period, &pool_addr)
}

/// Manages contract migration
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Err(ContractError::MigrationError {})
}
