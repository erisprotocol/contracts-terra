use std::collections::HashSet;
use std::convert::TryInto;

use astroport::common::{claim_ownership, drop_ownership_proposal, propose_new_owner};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Addr, Attribute, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdError, StdResult, Storage, Uint128,
};
use cw2::{get_contract_version, set_contract_version};
use cw_storage_plus::Bound;
use eris::hub::get_hub_validators;
use itertools::Itertools;

use eris::amp_gauges::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, UserInfoResponse, UserInfosResponse,
    VotedValidatorInfoResponse,
};
use eris::governance_helper::{calc_voting_power, get_period};
use eris::helpers::bps::BasicPoints;
use eris::voting_escrow::{get_lock_info, LockInfoResponse, DEFAULT_LIMIT, MAX_LIMIT};

use crate::error::ContractError;
use crate::state::{
    Config, TuneInfo, UserInfo, CONFIG, OWNERSHIP_PROPOSAL, TUNE_INFO, USER_INFO, VALIDATORS,
};
use crate::utils::{
    add_fixed_vamp, cancel_user_changes, fetch_last_validator_fixed_vamp_value, filter_validators,
    get_validator_info, remove_fixed_vamp, update_validator_info, vote_for_validator,
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
            owner: deps.api.addr_validate(&msg.owner)?,
            escrow_addr: deps.api.addr_validate(&msg.escrow_addr)?,
            hub_addr: deps.api.addr_validate(&msg.hub_addr)?,
            validators_limit: msg.validators_limit,
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
        ExecuteMsg::RemoveUser {
            user,
        } => remove_user(deps, env, info, user),
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

    let ve_lock_info = get_lock_info(&deps.querier, &config.escrow_addr, &user)?;
    let vamp = ve_lock_info.voting_power + ve_lock_info.fixed_amount;
    if vamp.is_zero() {
        return Err(ContractError::ZeroVotingPower {});
    }

    let user_info = USER_INFO.may_load(deps.storage, &user)?.unwrap_or_default();

    // Check duplicated votes
    let addrs_set = votes.iter().cloned().map(|(addr, _)| addr).collect::<HashSet<_>>();
    if votes.len() != addrs_set.len() {
        return Err(ContractError::DuplicatedValidators {});
    }

    let allowed_validators = get_hub_validators(&deps.querier, config.hub_addr)?;

    // Validating addrs and bps
    let votes = votes
        .into_iter()
        .map(|(addr, bps)| {
            if !allowed_validators.contains(&addr) {
                return Err(ContractError::InvalidValidatorAddress(addr));
            }
            let bps: BasicPoints = bps.try_into()?;
            Ok((addr, bps))
        })
        .collect::<Result<Vec<_>, ContractError>>()?;

    // Check the bps sum is within the limit
    votes.iter().try_fold(BasicPoints::default(), |acc, (_, bps)| acc.checked_add(*bps))?;

    remove_votes_of_user(&user_info, block_period, deps.storage)?;

    apply_votest_of_user(
        votes,
        deps,
        block_period,
        ve_lock_info.voting_power,
        ve_lock_info,
        env,
        user,
    )?;

    Ok(Response::new().add_attribute("action", "vamp/vote").add_attribute("vAMP", vamp))
}

fn apply_votest_of_user(
    votes: Vec<(String, BasicPoints)>,
    deps: DepsMut,
    block_period: u64,
    user_vp: Uint128,
    ve_lock_info: LockInfoResponse,
    env: Env,
    user: Addr,
) -> Result<(), ContractError> {
    votes.iter().try_for_each(|(validator_addr, bps)| {
        add_fixed_vamp(
            deps.storage,
            block_period + 1,
            validator_addr,
            *bps * ve_lock_info.fixed_amount,
        )?;
        vote_for_validator(
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
        fixed_amount: ve_lock_info.fixed_amount,
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
        let user_last_vote_period = get_period(user_info.vote_ts)?;
        // Calculate voting power before changes
        let old_vp_at_period = calc_voting_power(
            user_info.slope,
            user_info.voting_power,
            user_last_vote_period,
            block_period,
        );

        // Cancel changes applied by previous votes
        user_info.votes.iter().try_for_each(|(validator_addr, bps)| {
            remove_fixed_vamp(
                storage,
                block_period + 1,
                validator_addr,
                *bps * user_info.fixed_amount,
            )?;
            cancel_user_changes(
                storage,
                block_period + 1,
                validator_addr,
                *bps,
                old_vp_at_period,
                user_info.slope,
                user_info.lock_end,
            )
        })?;
    } else {
        // still need to remove fixed vamp on remove
        user_info.votes.iter().try_for_each(|(validator_addr, bps)| {
            remove_fixed_vamp(
                storage,
                block_period + 1,
                validator_addr,
                *bps * user_info.fixed_amount,
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
    lock: LockInfoResponse,
) -> ExecuteResult {
    let block_period = get_period(env.block.time.seconds())?;
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.escrow_addr {
        return Err(ContractError::Unauthorized {});
    }

    let user = deps.api.addr_validate(&user)?;
    let user_info = USER_INFO.may_load(deps.storage, &user)?;

    if let Some(user_info) = user_info {
        remove_votes_of_user(&user_info, block_period, deps.storage)?;

        if lock.voting_power.is_zero() && lock.fixed_amount.is_zero() {
            let user_info = UserInfo {
                vote_ts: env.block.time.seconds(),
                voting_power: Uint128::zero(),
                slope: lock.slope,
                lock_end: lock.end,
                fixed_amount: lock.fixed_amount,
                votes: user_info.votes,
            };
            USER_INFO.save(deps.storage, &user, &user_info)?;

            return Ok(Response::new().add_attribute("action", "vamp/update_vote_removed"));
        }

        let vamp = lock.voting_power + lock.fixed_amount;
        apply_votest_of_user(
            user_info.votes,
            deps,
            block_period,
            lock.voting_power,
            lock,
            env,
            user,
        )?;

        return Ok(Response::new()
            .add_attribute("action", "vamp/update_vote_changed")
            .add_attribute("vAMP", vamp));
    }

    Ok(Response::new().add_attribute("action", "vamp/update_vote_noop"))
}

fn remove_user(deps: DepsMut, env: Env, info: MessageInfo, user: String) -> ExecuteResult {
    let config = CONFIG.load(deps.storage)?;
    config.assert_owner(&info.sender)?;

    let user = deps.api.addr_validate(&user)?;
    let user_info = USER_INFO.may_load(deps.storage, &user)?;

    if let Some(user_info) = user_info {
        let block_period = get_period(env.block.time.seconds())?;
        USER_INFO.remove(deps.storage, &user);

        let result = remove_votes_of_user(&user_info, block_period, deps.storage);
        let msg = if let Err(err) = result {
            err.to_string()
        } else {
            "ok".to_string()
        };
        return Ok(Response::new()
            .add_attribute("action", "vamp/remove_user")
            .add_attribute("remove_votes", msg));
    }

    Ok(Response::new().add_attribute("action", "vamp/remove_user_noop"))
}

/// The function checks that the last pools tuning happened >= 14 days ago.
/// Then it calculates voting power for each pool at the current period, filters all pools which
/// are not eligible to receive allocation points,
/// takes top X pools by voting power, where X is 'config.pools_limit', calculates allocation points
/// for these pools and applies allocation points in generator contract.
fn tune_vamp(deps: DepsMut, env: Env, info: MessageInfo) -> ExecuteResult {
    let config = CONFIG.load(deps.storage)?;
    config.assert_owner(&info.sender)?;

    let block_period = get_period(env.block.time.seconds())?;

    let validator_votes: Vec<_> = VALIDATORS
        .keys(deps.as_ref().storage, None, None, Order::Ascending)
        .collect::<Vec<_>>()
        .into_iter()
        .map(|validator_addr| {
            let validator_addr = validator_addr?;

            let validator_info =
                update_validator_info(deps.storage, block_period, &validator_addr, None)?;

            let vamp = validator_info.voting_power.checked_add(
                fetch_last_validator_fixed_vamp_value(deps.storage, block_period, &validator_addr)?,
            )?;

            // Remove pools with zero voting power so we won't iterate over them in future
            if vamp.is_zero()
            // and the next period is also unset
                && fetch_last_validator_fixed_vamp_value(
                    deps.storage,
                    block_period + 1,
                    &validator_addr,
                )?
                .is_zero()
            {
                VALIDATORS.remove(deps.storage, &validator_addr)
            }
            Ok((validator_addr, vamp))
        })
        .collect::<StdResult<Vec<_>>>()?
        .into_iter()
        .filter(|(_, vamp_amount)| !vamp_amount.is_zero())
        .sorted_by(|(_, a), (_, b)| b.cmp(a)) // Sort in descending order
        .collect();

    let mut tune_info = TUNE_INFO.load(deps.storage)?;
    tune_info.vamp_points = filter_validators(
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

    let attributes: Vec<Attribute> =
        tune_info.vamp_points.iter().map(|a| attr("vamp", format!("{0}={1}", a.0, a.1))).collect();

    Ok(Response::new().add_attribute("action", "vamp/tune_vamp").add_attributes(attributes))
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

    Ok(Response::default().add_attribute("action", "vamp/update_config"))
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
        } => to_binary(&user_info(deps, env, user)?),
        QueryMsg::UserInfos {
            start_after,
            limit,
        } => to_binary(&user_infos(deps, env, start_after, limit)?),
        QueryMsg::TuneInfo {} => to_binary(&TUNE_INFO.load(deps.storage)?),
        QueryMsg::Config {} => to_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::ValidatorInfo {
            validator_addr,
        } => to_binary(&validator_info(deps, env, validator_addr, None)?),
        QueryMsg::ValidatorInfos {
            period,
            validator_addrs,
        } => to_binary(&validator_infos(deps, env, validator_addrs, period)?),
        QueryMsg::ValidatorInfoAtPeriod {
            validator_addr,
            period,
        } => to_binary(&validator_info(deps, env, validator_addr, Some(period))?),
    }
}

/// Returns user information.
fn user_info(deps: Deps, env: Env, user: String) -> StdResult<UserInfoResponse> {
    let user_addr = deps.api.addr_validate(&user)?;
    let user = USER_INFO
        .may_load(deps.storage, &user_addr)?
        .ok_or_else(|| StdError::generic_err("User not found"))?;

    let block_period = get_period(env.block.time.seconds())?;
    UserInfo::into_response(user, block_period)
}

// returns all user votes
fn user_infos(
    deps: Deps,
    env: Env,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<UserInfosResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let mut start: Option<Bound<&Addr>> = None;
    let addr: Addr;
    if let Some(start_after) = start_after {
        if let Ok(start_after_addr) = deps.api.addr_validate(&start_after) {
            addr = start_after_addr;
            start = Some(Bound::exclusive(&addr));
        }
    }

    let block_period = get_period(env.block.time.seconds())?;

    let users = USER_INFO
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (user, v) = item?;
            Ok((user, UserInfo::into_response(v, block_period)?))
        })
        .collect::<StdResult<Vec<(Addr, UserInfoResponse)>>>()?;

    Ok(UserInfosResponse {
        users,
    })
}

/// Returns all active validators info at a specified period.
fn validator_infos(
    deps: Deps,
    env: Env,
    validator_addrs: Option<Vec<String>>,
    period: Option<u64>,
) -> StdResult<Vec<(String, VotedValidatorInfoResponse)>> {
    let period = period.unwrap_or(get_period(env.block.time.seconds())?);

    // use active validators as fallback
    let validator_addrs = validator_addrs.unwrap_or_else(|| {
        let active_validators = VALIDATORS
            .keys(deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>();

        active_validators.unwrap_or_default()
    });

    let validator_infos: Vec<_> = validator_addrs
        .into_iter()
        .map(|validator_addr| {
            let validator_info = get_validator_info(deps.storage, period, &validator_addr)?;
            Ok((validator_addr, validator_info))
        })
        .collect::<StdResult<Vec<_>>>()?;

    Ok(validator_infos)
}

/// Returns pool's voting information at a specified period.
fn validator_info(
    deps: Deps,
    env: Env,
    validator_addr: String,
    period: Option<u64>,
) -> StdResult<VotedValidatorInfoResponse> {
    let block_period = get_period(env.block.time.seconds())?;
    let period = period.unwrap_or(block_period);
    get_validator_info(deps.storage, period, &validator_addr)
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
