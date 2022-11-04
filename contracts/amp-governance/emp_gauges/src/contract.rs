use std::collections::HashSet;

use astroport::asset::addr_validate_to_lower;
use astroport::common::{claim_ownership, drop_ownership_proposal, propose_new_owner};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Attribute, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdError,
    StdResult, Uint128,
};
use cw2::set_contract_version;
use eris::helpers::sloap::adjust_vp_and_slope;
use eris::hub::get_hub_validators;
use itertools::Itertools;

use crate::error::ContractError;
use crate::state::{
    Config, TuneInfo, VotedValidatorInfo, CONFIG, OWNERSHIP_PROPOSAL, TUNE_INFO, VALIDATORS,
};
use crate::utils::{
    add_fixed_emp, fetch_current_validator_fixed_emps_value, filter_validators, get_validator_info,
    update_validator_info, vote_for_validator,
};
use eris::emp_gauges::{
    get_tune_msg, AddEmpInfo, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};

use eris::governance_helper::get_period;

/// Contract name that is used for migration.
const CONTRACT_NAME: &str = "emp-gauges";
/// Contract version that is used for migration.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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
            hub_addr: addr_validate_to_lower(deps.api, &msg.hub_addr)?,
            validators_limit: msg.validators_limit,
        },
    )?;

    // Set tune_ts just for safety so the first tuning could happen in 2 weeks
    TUNE_INFO.save(
        deps.storage,
        &TuneInfo {
            tune_ts: env.block.time.seconds(),
            emp_points: vec![],
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
        ExecuteMsg::AddEmps {
            emps,
        } => handle_vote(deps, env, info, emps),
        ExecuteMsg::TuneEmps {} => tune_pools(deps, env, info),
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
/// * user didn't vote for last 10 days,
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
    validator_emps: Vec<AddEmpInfo>,
) -> ExecuteResult {
    let config = CONFIG.load(deps.storage)?;
    config.assert_owner(&info.sender)?;

    let block_period = get_period(env.block.time.seconds())?;

    // Check duplicated votes
    let addrs_set = validator_emps.iter().cloned().map(|(addr, _)| addr).collect::<HashSet<_>>();
    if validator_emps.len() != addrs_set.len() {
        return Err(ContractError::DuplicatedValidators {});
    }

    let validators = get_hub_validators(&deps.querier, config.hub_addr)?;

    for emp in validator_emps {
        let (addr, added_points) = emp;

        if !validators.contains(&addr) {
            return Err(ContractError::InvalidValidatorAddress(addr));
        }

        let validator_addr = addr_validate_to_lower(deps.api, &addr)?;

        added_points.iter().try_for_each(|emp| -> StdResult<()> {
            if let Some(decaying_periods) = emp.decaying_period {
                let dt = decaying_periods;

                let end = block_period + dt;
                let mut add_voting_power = emp.umerit_points;
                let slope = adjust_vp_and_slope(&mut add_voting_power, dt, Uint128::zero())?;

                vote_for_validator(
                    deps.storage,
                    block_period,
                    &validator_addr,
                    add_voting_power,
                    slope,
                    end,
                )?;
            } else {
                add_fixed_emp(deps.storage, block_period, &validator_addr, emp.umerit_points)?;
            }

            Ok(())
        })?;
    }

    Ok(Response::new()
        .add_message(get_tune_msg(env.contract.address.to_string())?)
        .add_attribute("action", "vote"))
}

/// The function checks that the last pools tuning happened >= 14 days ago.
/// Then it calculates voting power for each pool at the current period, filters all pools which
/// are not eligible to receive allocation points,
/// takes top X pools by voting power, where X is 'config.pools_limit', calculates allocation points
/// for these pools and applies allocation points in generator contract.
fn tune_pools(deps: DepsMut, env: Env, info: MessageInfo) -> ExecuteResult {
    let config = CONFIG.load(deps.storage)?;
    config.assert_owner_or_self(&info.sender, &env.contract.address)?;

    let mut tune_info = TUNE_INFO.load(deps.storage)?;
    let block_period = get_period(env.block.time.seconds())?;

    let validator_votes: Vec<_> = VALIDATORS
        .keys(deps.as_ref().storage, None, None, Order::Ascending)
        .collect::<Vec<_>>()
        .into_iter()
        .map(|validator_addr| {
            let validator_addr = validator_addr?;

            let mut validator_info =
                update_validator_info(deps.storage, block_period, &validator_addr, None)?;

            validator_info.emp_amount =
                validator_info.emp_amount.checked_add(fetch_current_validator_fixed_emps_value(
                    deps.storage,
                    block_period,
                    &validator_addr,
                )?)?;
            // Remove pools with zero voting power so we won't iterate over them in future
            if validator_info.emp_amount.is_zero() {
                VALIDATORS.remove(deps.storage, &validator_addr)
            }
            Ok((validator_addr, validator_info.emp_amount))
        })
        .collect::<StdResult<Vec<_>>>()?
        .into_iter()
        .filter(|(_, emp_amount)| !emp_amount.is_zero())
        .sorted_by(|(_, a), (_, b)| b.cmp(a)) // Sort in descending order
        .collect();

    tune_info.emp_points = filter_validators(
        &deps.querier,
        &config.hub_addr,
        validator_votes,
        config.validators_limit,
    )?;

    if tune_info.emp_points.is_empty() {
        return Err(ContractError::TuneNoValidators {});
    }

    tune_info.tune_ts = env.block.time.seconds();
    TUNE_INFO.save(deps.storage, &tune_info)?;

    let attributes: Vec<Attribute> =
        tune_info.emp_points.iter().map(|a| attr("emps", format!("{0}={1}", a.0, a.1))).collect();

    Ok(Response::new()
        .add_attribute("action", "tune_pools")
        .add_attribute("period", block_period.to_string())
        .add_attributes(attributes))
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

/// Returns pool's voting information at a specified period.
fn validator_info(
    deps: Deps,
    env: Env,
    validator_addr: String,
    period: Option<u64>,
) -> StdResult<VotedValidatorInfo> {
    let validator_addr = addr_validate_to_lower(deps.api, &validator_addr)?;
    let block_period = get_period(env.block.time.seconds())?;
    let period = period.unwrap_or(block_period);
    get_validator_info(deps.storage, period, &validator_addr)
}

/// Manages contract migration
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Err(ContractError::MigrationError {})
}
