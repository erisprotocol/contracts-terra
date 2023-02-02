use std::collections::HashSet;

use astroport::common::{claim_ownership, drop_ownership_proposal, propose_new_owner};
use eris::helper::{addr_opt_validate, validate_addresses};
use eris::DecimalCheckedOps;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Response, StdError, StdResult, Storage, Uint128, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::{
    BalanceResponse, Cw20ExecuteMsg, Cw20ReceiveMsg, Logo, LogoInfo, MarketingInfoResponse,
    TokenInfoResponse,
};
use cw20_base::contract::{
    execute_update_marketing, execute_upload_logo, query_download_logo, query_marketing_info,
};
use cw20_base::state::{MinterData, TokenInfo, LOGO, MARKETING_INFO, TOKEN_INFO};

use eris::governance_helper::{get_period, get_periods_count, EPOCH_START, MIN_LOCK_PERIODS, WEEK};
use eris::helpers::slope::{adjust_vp_and_slope, calc_coefficient};
use eris::voting_escrow::{
    BlacklistedVotersResponse, ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg,
    LockInfoResponse, MigrateMsg, PushExecuteMsg, QueryMsg, VotingPowerResponse, DEFAULT_LIMIT,
    MAX_LIMIT,
};

use crate::error::ContractError;
use crate::marketing_validation::{validate_marketing_info, validate_whitelist_links};
use crate::state::{
    Config, Lock, Point, BLACKLIST, CONFIG, HISTORY, LAST_SLOPE_CHANGE, LOCKED, OWNERSHIP_PROPOSAL,
};
use crate::utils::{
    assert_blacklist, assert_periods_remaining, assert_time_limits, assert_token,
    calc_voting_power, cancel_scheduled_slope, fetch_last_checkpoint, fetch_slope_changes,
    schedule_slope_change,
};

/// Contract name that is used for migration.
const CONTRACT_NAME: &str = "eris-voting-escrow";
/// Contract version that is used for migration.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Creates a new contract with the specified parameters in [`InstantiateMsg`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let deposit_token_addr = deps.api.addr_validate(&msg.deposit_token_addr)?;

    validate_whitelist_links(&msg.logo_urls_whitelist)?;
    let guardian_addr = addr_opt_validate(deps.api, &msg.guardian_addr)?;

    let config = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
        guardian_addr,
        deposit_token_addr,
        logo_urls_whitelist: msg.logo_urls_whitelist.clone(),
        // makes no sense to set during init, as other contracts might not be deployed yet.
        push_update_contracts: vec![],
    };
    CONFIG.save(deps.storage, &config)?;

    let cur_period = get_period(env.block.time.seconds())?;
    let point = Point {
        power: Uint128::zero(),
        start: cur_period,
        end: 0,
        slope: Default::default(),
        fixed: Uint128::zero(),
    };
    HISTORY.save(deps.storage, (env.contract.address.clone(), cur_period), &point)?;
    BLACKLIST.save(deps.storage, &vec![])?;

    if let Some(marketing) = msg.marketing {
        if msg.logo_urls_whitelist.is_empty() {
            return Err(ContractError::MarketingInfoValidationError(
                "Logo URLs whitelist can not be empty".to_string(),
            ));
        }

        validate_marketing_info(
            marketing.project.as_ref(),
            marketing.description.as_ref(),
            marketing.logo.as_ref(),
            &config.logo_urls_whitelist,
        )?;

        let logo = if let Some(logo) = marketing.logo {
            LOGO.save(deps.storage, &logo)?;

            match logo {
                Logo::Url(url) => Some(LogoInfo::Url(url)),
                Logo::Embedded(_) => Some(LogoInfo::Embedded),
            }
        } else {
            None
        };

        let data = MarketingInfoResponse {
            project: marketing.project,
            description: marketing.description,
            marketing: addr_opt_validate(deps.api, &marketing.marketing)?,
            logo,
        };
        MARKETING_INFO.save(deps.storage, &data)?;
    }

    // Store token info
    let data = TokenInfo {
        name: "Vote Escrowed ampLP".to_string(),
        symbol: "vAMP".to_string(),
        decimals: 6,
        total_supply: Uint128::zero(),
        mint: Some(MinterData {
            minter: env.contract.address,
            cap: None,
        }),
    };

    TOKEN_INFO.save(deps.storage, &data)?;

    Ok(Response::default())
}

/// Exposes all the execute functions available in the contract.
///
/// ## Execute messages
/// * **ExecuteMsg::ExtendLockTime { time }** Increase a staker's lock time.
///
/// * **ExecuteMsg::Receive(msg)** Parse incoming messages coming from the ampLP token contract.
///
/// * **ExecuteMsg::Withdraw {}** Withdraw all ampLP from a lock position if the lock has expired.
///
/// * **ExecuteMsg::ProposeNewOwner { owner, expires_in }** Creates a new request to change contract ownership.
///
/// * **ExecuteMsg::DropOwnershipProposal {}** Removes a request to change contract ownership.
///
/// * **ExecuteMsg::ClaimOwnership {}** Claims contract ownership.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ExtendLockTime {
            time,
        } => extend_lock_time(deps, env, info, time),
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::Withdraw {} => withdraw(deps, env, info),
        ExecuteMsg::ProposeNewOwner {
            new_owner,
            expires_in,
        } => {
            let config = CONFIG.load(deps.storage)?;
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
        ExecuteMsg::UpdateBlacklist {
            append_addrs,
            remove_addrs,
        } => update_blacklist(deps, env, info, append_addrs, remove_addrs),
        ExecuteMsg::UpdateMarketing {
            project,
            description,
            marketing,
        } => {
            validate_marketing_info(project.as_ref(), description.as_ref(), None, &[])?;
            execute_update_marketing(deps, env, info, project, description, marketing)
                .map_err(Into::into)
        },
        ExecuteMsg::UploadLogo(logo) => {
            let config = CONFIG.load(deps.storage)?;
            validate_marketing_info(None, None, Some(&logo), &config.logo_urls_whitelist)?;
            execute_upload_logo(deps, env, info, logo).map_err(Into::into)
        },
        ExecuteMsg::SetLogoUrlsWhitelist {
            whitelist,
        } => {
            let mut config = CONFIG.load(deps.storage)?;
            let marketing_info = MARKETING_INFO.load(deps.storage)?;
            if info.sender != config.owner && Some(info.sender) != marketing_info.marketing {
                Err(ContractError::Unauthorized {})
            } else {
                validate_whitelist_links(&whitelist)?;
                config.logo_urls_whitelist = whitelist;
                CONFIG.save(deps.storage, &config)?;
                Ok(Response::default().add_attribute("action", "veamp/set_logo_urls_whitelist"))
            }
        },
        ExecuteMsg::UpdateConfig {
            new_guardian,
            push_update_contracts,
        } => execute_update_config(deps, info, new_guardian, push_update_contracts),
    }
}

/// Checkpoint the total voting power (total supply of vAMP).
/// This function fetches the last available vAMP checkpoint, recalculates passed periods since the checkpoint and until now,
/// applies slope changes and saves all recalculated periods in [`HISTORY`].
///
/// * **add_voting_power** amount of vAMP to add to the total.
///
/// * **reduce_power** amount of vAMP to subtract from the total.
///
/// * **old_slope** old slope applied to the total voting power (vAMP supply).
///
/// * **new_slope** new slope to be applied to the total voting power (vAMP supply).
#[allow(clippy::too_many_arguments)]
fn checkpoint_total(
    storage: &mut dyn Storage,
    env: Env,
    add_voting_power: Option<Uint128>,
    add_amount: Option<Uint128>,
    reduce_power: Option<Uint128>,
    reduce_amount: Option<Uint128>,
    old_slope: Uint128,
    new_slope: Uint128,
) -> Result<(), ContractError> {
    let cur_period = get_period(env.block.time.seconds())?;
    let cur_period_key = cur_period;
    let contract_addr = env.contract.address;
    let add_voting_power = add_voting_power.unwrap_or_default();
    let add_amount = add_amount.unwrap_or_default();

    // Get last checkpoint
    let last_checkpoint = fetch_last_checkpoint(storage, &contract_addr, cur_period_key)?;
    let new_point = if let Some((_, mut point)) = last_checkpoint {
        let last_slope_change = LAST_SLOPE_CHANGE.may_load(storage)?.unwrap_or(0);
        if last_slope_change < cur_period {
            let scheduled_slope_changes =
                fetch_slope_changes(storage, last_slope_change, cur_period)?;
            // Recalculating passed points
            for (recalc_period, scheduled_change) in scheduled_slope_changes {
                point = Point {
                    power: calc_voting_power(&point, recalc_period),
                    start: recalc_period,
                    slope: point.slope.checked_sub(scheduled_change).map_err(|orig| {
                        ContractError::OverflowLocation {
                            location: "checkpoint_total:recalculating".into(),
                            orig,
                        }
                    })?,
                    ..point
                };
                HISTORY.save(storage, (contract_addr.clone(), recalc_period), &point)?
            }

            LAST_SLOPE_CHANGE.save(storage, &cur_period)?
        }

        let new_power = (calc_voting_power(&point, cur_period) + add_voting_power)
            .saturating_sub(reduce_power.unwrap_or_default());

        Point {
            power: new_power,
            slope: point.slope.checked_sub(old_slope).map_err(|orig| {
                ContractError::OverflowLocation {
                    location: "checkpoint_total:new_point".into(),
                    orig,
                }
            })? + new_slope,
            start: cur_period,
            fixed: (point.fixed + add_amount)
                .checked_sub(reduce_amount.unwrap_or_default())
                .unwrap_or_default(),
            ..point
        }
    } else {
        Point {
            power: add_voting_power,
            slope: new_slope,
            start: cur_period,
            end: 0, // we don't use 'end' in total voting power calculations
            fixed: add_amount,
        }
    };
    HISTORY.save(storage, (contract_addr, cur_period_key), &new_point)?;
    Ok(())
}

/// Checkpoint a user's voting power (vAMP balance).
/// This function fetches the user's last available checkpoint, calculates the user's current voting power, applies slope changes based on
/// `add_amount` and `new_end` parameters, schedules slope changes for total voting power and saves the new checkpoint for the current
/// period in [`HISTORY`] (using the user's address).
/// If a user already checkpointed themselves for the current period, then this function uses the current checkpoint as the latest
/// available one.
///
/// * **addr** staker for which we checkpoint the voting power.
///
/// * **add_amount** amount of vAMP to add to the staker's balance.
///
/// * **new_end** new lock time for the staker's vAMP position.
fn checkpoint(
    store: &mut dyn Storage,
    env: Env,
    addr: Addr,
    add_amount: Option<Uint128>,
    new_end: Option<u64>,
) -> Result<(), ContractError> {
    let cur_period = get_period(env.block.time.seconds())?;
    let cur_period_key = cur_period;
    let add_amount = add_amount.unwrap_or_default();
    let mut old_slope = Default::default();
    let mut add_voting_power = Uint128::zero();

    // Get the last user checkpoint
    let last_checkpoint = fetch_last_checkpoint(store, &addr, cur_period_key)?;
    let new_point = if let Some((_, point)) = last_checkpoint {
        let end = new_end.unwrap_or(point.end);
        let dt = end.saturating_sub(cur_period);
        let current_power = calc_voting_power(&point, cur_period);

        let new_slope = if dt != 0 {
            // always recalculate slope when the end has changed
            if end > point.end {
                // This is extend_lock_time. Recalculating user's voting power
                let mut lock = LOCKED.load(store, addr.clone())?;
                let mut new_voting_power = calc_coefficient(dt).checked_mul_uint(lock.amount)?;
                let slope = adjust_vp_and_slope(&mut new_voting_power, dt)?; // end_vp
                                                                             // new_voting_power should always be >= current_power. saturating_sub is used for extra safety
                add_voting_power = new_voting_power.saturating_sub(current_power);
                lock.last_extend_lock_period = cur_period;
                LOCKED.save(store, addr.clone(), &lock, env.block.height)?;
                slope
            } else {
                // This is an increase in the user's lock amount
                let raw_add_voting_power = calc_coefficient(dt).checked_mul_uint(add_amount)?;
                let mut new_voting_power = current_power.checked_add(raw_add_voting_power)?;
                let slope = adjust_vp_and_slope(&mut new_voting_power, dt)?;
                // new_voting_power should always be >= current_power. saturating_sub is used for extra safety
                add_voting_power = new_voting_power.saturating_sub(current_power);
                slope
            }
        } else {
            Uint128::zero()
        };

        // Cancel the previously scheduled slope change (same logic as in cancel_scheduled_slope)
        let last_slope_change = cancel_scheduled_slope(store, point.slope, point.end)?;

        if point.end > last_slope_change {
            // We need to subtract the slope point from the total voting power slope
            // Only if the point is still active and has not been processed/applied yet.
            old_slope = point.slope
        };

        Point {
            power: current_power + add_voting_power,
            slope: new_slope,
            start: cur_period,
            end,
            fixed: point.fixed + add_amount,
        }
    } else {
        // This error can't happen since this if-branch is intended for checkpoint creation
        let end = new_end.ok_or(ContractError::CheckpointInitializationFailed {})?;
        let dt = end - cur_period;
        add_voting_power = calc_coefficient(dt).checked_mul_uint(add_amount)?;
        let slope = adjust_vp_and_slope(&mut add_voting_power, dt)?; //add_amount
        Point {
            power: add_voting_power,
            slope,
            start: cur_period,
            end,
            fixed: add_amount,
        }
    };

    // Schedule a slope change
    schedule_slope_change(store, new_point.slope, new_point.end)?;

    HISTORY.save(store, (addr, cur_period_key), &new_point)?;

    checkpoint_total(
        store,
        env,
        Some(add_voting_power),
        Some(add_amount),
        None,
        None,
        old_slope,
        new_point.slope,
    )
}

/// Receives a message of type [`Cw20ReceiveMsg`] and processes it depending on the received template.
///
/// * **cw20_msg** CW20 message to process.
fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    assert_token(deps.storage, info.sender)?;
    let sender = deps.api.addr_validate(&cw20_msg.sender)?;
    assert_blacklist(deps.storage, &sender)?;

    match from_binary(&cw20_msg.msg)? {
        Cw20HookMsg::CreateLock {
            time,
        } => create_lock(deps, env, sender, cw20_msg.amount, time),
        Cw20HookMsg::ExtendLockAmount {
            extend_to_min_periods,
        } => deposit_for(deps, env, cw20_msg.amount, sender, extend_to_min_periods),
        Cw20HookMsg::DepositFor {
            user,
        } => {
            let addr = deps.api.addr_validate(&user)?;
            assert_blacklist(deps.storage, &addr)?;
            deposit_for(deps, env, cw20_msg.amount, addr, None)
        },
    }
}

/// Creates a lock for the user that lasts for the specified time duration (in seconds).
/// Checks that the user is locking ampLP tokens.
/// Checks that the lock time is within [`WEEK`]..[`MAX_LOCK_TIME`].
/// Creates a lock if it doesn't exist and triggers a [`checkpoint`] for the staker.
/// If a lock already exists, then a [`ContractError`] is returned.
///
/// * **user** staker for which we create a lock position.
///
/// * **amount** amount of ampLP deposited in the lock position.
///
/// * **time** duration of the lock.
fn create_lock(
    deps: DepsMut,
    env: Env,
    user: Addr,
    amount: Uint128,
    time: u64,
) -> Result<Response, ContractError> {
    assert_time_limits(time)?;

    let block_period = get_period(env.block.time.seconds())?;
    let periods = get_periods_count(time);
    let end = block_period + periods;

    assert_periods_remaining(periods)?;

    LOCKED.update(deps.storage, user.clone(), env.block.height, |lock_opt| {
        if lock_opt.is_some() && !lock_opt.unwrap().amount.is_zero() {
            return Err(ContractError::LockAlreadyExists {});
        }
        Ok(Lock {
            amount,
            start: block_period,
            end,
            last_extend_lock_period: block_period,
        })
    })?;

    checkpoint(deps.storage, env.clone(), user.clone(), Some(amount), Some(end))?;

    let config = CONFIG.load(deps.storage)?;

    let lock_info = get_user_lock_info(deps.as_ref(), &env, user.to_string())?;

    Ok(Response::default()
        .add_attribute("action", "veamp/create_lock")
        .add_attribute("voting_power", lock_info.voting_power.to_string())
        .add_attribute("fixed_power", lock_info.fixed_amount.to_string())
        .add_attribute("lock_end", lock_info.end.to_string())
        .add_messages(get_push_update_msgs(config, user, Ok(lock_info))?))
}

/// Deposits an 'amount' of ampLP tokens into 'user''s lock.
/// Checks that the user is transferring and locking ampLP.
/// Triggers a [`checkpoint`] for the user.
/// If the user does not have a lock, then a [`ContractError`] is returned.
///
/// * **amount** amount of ampLP to deposit.
///
/// * **user** user who's lock amount will increase.
fn deposit_for(
    deps: DepsMut,
    env: Env,
    amount: Uint128,
    user: Addr,
    extend_to_min_periods: Option<bool>,
) -> Result<Response, ContractError> {
    let mut new_end = None;
    LOCKED.update(deps.storage, user.clone(), env.block.height, |lock_opt| match lock_opt {
        Some(mut lock) if !lock.amount.is_zero() => {
            let block_period = get_period(env.block.time.seconds())?;

            match extend_to_min_periods {
                Some(true) => {
                    if lock.end < block_period + MIN_LOCK_PERIODS {
                        lock.end = block_period + MIN_LOCK_PERIODS;
                        new_end = Some(lock.end);
                    }
                },
                Some(false) | None => {
                    if lock.end <= block_period {
                        Err(ContractError::LockExpired {})?
                    }
                    assert_periods_remaining(lock.end - block_period)?
                },
            }

            lock.amount += amount;
            Ok(lock)
        },
        _ => Err(ContractError::LockDoesNotExist {}),
    })?;

    checkpoint(deps.storage, env.clone(), user.clone(), Some(amount), new_end)?;

    let config = CONFIG.load(deps.storage)?;

    let lock_info = get_user_lock_info(deps.as_ref(), &env, user.to_string())?;

    Ok(Response::default()
        .add_attribute("action", "veamp/deposit_for")
        .add_attribute("voting_power", lock_info.voting_power.to_string())
        .add_attribute("fixed_power", lock_info.fixed_amount.to_string())
        .add_attribute("lock_end", lock_info.end.to_string())
        .add_messages(get_push_update_msgs(config, user, Ok(lock_info))?))
}

/// Withdraws the whole amount of locked ampLP from a specific user lock.
/// If the user lock doesn't exist or if it has not yet expired, then a [`ContractError`] is returned.
fn withdraw(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let sender = info.sender;
    // 'LockDoesNotExist' is thrown either when a lock does not exist in LOCKED or when a lock exists but lock.amount == 0
    let mut lock = LOCKED
        .may_load(deps.storage, sender.clone())?
        .filter(|lock| !lock.amount.is_zero())
        .ok_or(ContractError::LockDoesNotExist {})?;

    let cur_period = get_period(env.block.time.seconds())?;
    if lock.end > cur_period {
        Err(ContractError::LockHasNotExpired {})
    } else {
        let config = CONFIG.load(deps.storage)?;
        let transfer_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.deposit_token_addr.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: sender.to_string(),
                amount: lock.amount,
            })?,
            funds: vec![],
        });
        let amount = lock.amount;
        lock.amount = Uint128::zero();
        LOCKED.save(deps.storage, sender.clone(), &lock, env.block.height)?;

        // We need to checkpoint and eliminate the slope influence on a future lock
        HISTORY.save(
            deps.storage,
            (sender.clone(), cur_period),
            &Point {
                power: Uint128::zero(),
                start: cur_period,
                end: cur_period,
                slope: Default::default(),
                fixed: Uint128::zero(),
            },
        )?;

        // removing funds needs to remove from total checkpoint aswell.
        checkpoint_total(
            deps.storage,
            env.clone(),
            None,
            None,
            None,
            Some(amount),
            Default::default(),
            Default::default(),
        )?;

        let lock_info = get_user_lock_info(deps.as_ref(), &env, sender.to_string());
        let msgs = get_push_update_msgs(config, sender, lock_info)?;

        Ok(Response::default()
            .add_message(transfer_msg)
            .add_messages(msgs)
            .add_attribute("action", "veamp/withdraw"))
    }
}

fn get_push_update_msgs_multi(
    deps: Deps,
    env: Env,
    config: Config,
    sender: Vec<Addr>,
) -> StdResult<Vec<CosmosMsg>> {
    let results: Vec<CosmosMsg> = sender
        .into_iter()
        .map(|sender| {
            let lock_info = get_user_lock_info(deps, &env, sender.to_string());
            get_push_update_msgs(config.clone(), sender, lock_info)
        })
        .collect::<StdResult<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();

    Ok(results)
}

fn get_push_update_msgs(
    config: Config,
    sender: Addr,
    lock_info: Result<LockInfoResponse, ContractError>,
) -> StdResult<Vec<CosmosMsg>> {
    // only send update if lock info is available. LOCK info is never removed for any user that locked anything.
    if let Ok(lock_info) = lock_info {
        config
            .push_update_contracts
            .into_iter()
            .map(|contract| {
                Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract.to_string(),
                    msg: to_binary(&PushExecuteMsg::UpdateVote {
                        user: sender.to_string(),
                        lock_info: lock_info.clone(),
                    })?,
                    funds: vec![],
                }))
            })
            .collect::<StdResult<Vec<_>>>()
    } else {
        Ok(vec![])
    }
}

/// Increase the current lock time for a staker by a specified time period.
/// Evaluates that the `time` is within [`WEEK`]..[`MAX_LOCK_TIME`]
/// and then it triggers a [`checkpoint`].
/// If the user lock doesn't exist or if it expired, then a [`ContractError`] is returned.
///
/// ## Note
/// The time is added to the lock's `end`.
/// For example, at period 0, the user has their ampLP locked for 3 weeks.
/// In 1 week, they increase their lock time by 10 weeks, thus the unlock period becomes 13 weeks.
///
/// * **time** increase in lock time applied to the staker's position.
fn extend_lock_time(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    time: u64,
) -> Result<Response, ContractError> {
    let user = info.sender;
    assert_blacklist(deps.storage, &user)?;
    let mut lock = LOCKED
        .may_load(deps.storage, user.clone())?
        .filter(|lock| !lock.amount.is_zero())
        .ok_or(ContractError::LockDoesNotExist {})?;

    // Disable the ability to extend the lock time by less than a week
    assert_time_limits(time)?;

    let block_period = get_period(env.block.time.seconds())?;
    if lock.end < block_period {
        // if the lock.end is in the past, extend_lock_time always starts from the current period.
        lock.end = block_period;
    };

    lock.end += get_periods_count(time);

    let periods = lock.end - block_period;
    assert_periods_remaining(periods)?;

    // Should not exceed MAX_LOCK_TIME
    assert_time_limits(EPOCH_START + lock.end * WEEK - env.block.time.seconds())?;

    LOCKED.save(deps.storage, user.clone(), &lock, env.block.height)?;

    checkpoint(deps.storage, env.clone(), user.clone(), None, Some(lock.end))?;

    let config = CONFIG.load(deps.storage)?;

    let lock_info = get_user_lock_info(deps.as_ref(), &env, user.to_string())?;

    Ok(Response::default()
        .add_attribute("action", "veamp/extend_lock_time")
        .add_attribute("voting_power", lock_info.voting_power.to_string())
        .add_attribute("fixed_power", lock_info.fixed_amount.to_string())
        .add_attribute("lock_end", lock_info.end.to_string())
        .add_messages(get_push_update_msgs(config, user, Ok(lock_info))?))
}

/// Update the staker blacklist. Whitelists addresses specified in 'remove_addrs'
/// and blacklists new addresses specified in 'append_addrs'. Nullifies staker voting power and
/// cancels their contribution in the total voting power (total vAMP supply).
///
/// * **append_addrs** array of addresses to blacklist.
///
/// * **remove_addrs** array of addresses to whitelist.
fn update_blacklist(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    append_addrs: Option<Vec<String>>,
    remove_addrs: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // Permission check
    if info.sender != config.owner && Some(info.sender) != config.guardian_addr {
        return Err(ContractError::Unauthorized {});
    }
    let append_addrs = append_addrs.unwrap_or_default();
    let remove_addrs = remove_addrs.unwrap_or_default();
    let blacklist = BLACKLIST.load(deps.storage)?;
    let append: Vec<_> = validate_addresses(deps.api, &append_addrs)?
        .into_iter()
        .filter(|addr| !blacklist.contains(addr))
        .collect();
    let remove: Vec<_> = validate_addresses(deps.api, &remove_addrs)?
        .into_iter()
        .filter(|addr| blacklist.contains(addr))
        .collect();

    if append.is_empty() && remove.is_empty() {
        return Err(ContractError::AddressBlacklistEmpty {});
    }

    let cur_period = get_period(env.block.time.seconds())?;
    let cur_period_key = cur_period;
    let mut reduce_total_vp = Uint128::zero(); // accumulator for decreasing total voting power
    let mut old_slopes = Uint128::zero(); // accumulator for old slopes
    let mut old_amount = Uint128::zero(); // accumulator for old amount

    let mut used_addr: HashSet<Addr> = HashSet::new();

    for addr in append.iter() {
        if !used_addr.insert(addr.clone()) {
            return Err(ContractError::AddressBlacklistDuplicated(addr.to_string()));
        }

        let last_checkpoint = fetch_last_checkpoint(deps.storage, addr, cur_period_key)?;
        if let Some((_, point)) = last_checkpoint {
            // We need to checkpoint with zero power and zero slope
            HISTORY.save(
                deps.storage,
                (addr.clone(), cur_period_key),
                &Point {
                    power: Uint128::zero(),
                    slope: Default::default(),
                    start: cur_period,
                    end: cur_period,
                    fixed: Uint128::zero(),
                },
            )?;

            let cur_power = calc_voting_power(&point, cur_period);
            // User's contribution is already zero. Skipping them
            if cur_power.is_zero() {
                continue;
            }

            // User's contribution in the total voting power calculation
            reduce_total_vp += cur_power;
            old_slopes += point.slope;
            old_amount += point.fixed;
            cancel_scheduled_slope(deps.storage, point.slope, point.end)?;
        }
    }

    if !reduce_total_vp.is_zero() || !old_slopes.is_zero() {
        // Trigger a total voting power recalculation
        checkpoint_total(
            deps.storage,
            env.clone(),
            None,
            None,
            Some(reduce_total_vp),
            Some(old_amount),
            old_slopes,
            Default::default(),
        )?;
    }

    for addr in remove.iter() {
        if !used_addr.insert(addr.clone()) {
            return Err(ContractError::AddressBlacklistDuplicated(addr.to_string()));
        }

        let lock_opt = LOCKED.may_load(deps.storage, addr.clone())?;
        if let Some(Lock {
            amount,
            end,
            ..
        }) = lock_opt
        {
            checkpoint(deps.storage, env.clone(), addr.clone(), Some(amount), Some(end))?;
        }
    }

    BLACKLIST.update(deps.storage, |blacklist| -> StdResult<Vec<Addr>> {
        let mut updated_blacklist: Vec<_> =
            blacklist.into_iter().filter(|addr| !remove.contains(addr)).collect();
        updated_blacklist.extend(append.clone());
        Ok(updated_blacklist)
    })?;

    let mut attrs = vec![attr("action", "veamp/update_blacklist")];
    if !append_addrs.is_empty() {
        attrs.push(attr("added_addresses", append_addrs.join(",")))
    }
    if !remove_addrs.is_empty() {
        attrs.push(attr("removed_addresses", remove_addrs.join(",")))
    }

    Ok(Response::default()
        .add_attributes(attrs)
        .add_messages(get_push_update_msgs_multi(
            deps.as_ref(),
            env.clone(),
            config.clone(),
            append,
        )?)
        .add_messages(get_push_update_msgs_multi(deps.as_ref(), env, config, remove)?))
}

/// Updates contracts' guardian address.
fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_guardian: Option<String>,
    push_update_contracts: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    let mut cfg = CONFIG.load(deps.storage)?;

    if cfg.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(new_guardian) = new_guardian {
        cfg.guardian_addr = Some(deps.api.addr_validate(&new_guardian)?);
    }

    if let Some(push_update_contracts) = push_update_contracts {
        cfg.push_update_contracts = push_update_contracts
            .iter()
            .map(|c| deps.api.addr_validate(c))
            .collect::<StdResult<Vec<_>>>()?;
    }

    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::default().add_attribute("action", "veamp/execute_update_config"))
}

/// Expose available contract queries.
///
/// ## Queries
/// * **QueryMsg::TotalVotingPower {}** Fetch the total voting power (vAMP supply) at the current block.
///
/// * **QueryMsg::UserVotingPower { user }** Fetch the user's voting power (vAMP balance) at the current block.
///
/// * **QueryMsg::TotalVotingPowerAt { time }** Fetch the total voting power (vAMP supply) at a specified timestamp.
///
/// * **QueryMsg::UserVotingPowerAt { time }** Fetch the user's voting power (vAMP balance) at a specified timestamp.
///
/// * **QueryMsg::LockInfo { user }** Fetch a user's lock information.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::CheckVotersAreBlacklisted {
            voters,
        } => Ok(to_binary(&check_voters_are_blacklisted(deps, voters)?)?),
        QueryMsg::BlacklistedVoters {
            start_after,
            limit,
        } => Ok(to_binary(&get_blacklisted_voters(deps, start_after, limit)?)?),
        QueryMsg::TotalVamp {} => Ok(to_binary(&get_total_vamp(deps, env, None)?)?),
        QueryMsg::UserVamp {
            user,
        } => Ok(to_binary(&get_user_vamp(deps, env, user, None)?)?),
        QueryMsg::TotalVampAt {
            time,
        } => Ok(to_binary(&get_total_vamp(deps, env, Some(time))?)?),
        QueryMsg::TotalVampAtPeriod {
            period,
        } => Ok(to_binary(&get_total_vamp_at_period(deps, env, period)?)?),
        QueryMsg::UserVampAt {
            user,
            time,
        } => Ok(to_binary(&get_user_vamp(deps, env, user, Some(time))?)?),
        QueryMsg::UserVampAtPeriod {
            user,
            period,
        } => Ok(to_binary(&get_user_vamp_at_period(deps, user, period)?)?),
        QueryMsg::LockInfo {
            user,
        } => Ok(to_binary(&get_user_lock_info(deps, &env, user)?)?),
        QueryMsg::UserDepositAtHeight {
            user,
            height,
        } => Ok(to_binary(&get_user_deposit_at_height(deps, user, height)?)?),
        QueryMsg::Config {} => {
            let config = CONFIG.load(deps.storage)?;
            Ok(to_binary(&ConfigResponse {
                owner: config.owner.to_string(),
                guardian_addr: config.guardian_addr,
                deposit_token_addr: config.deposit_token_addr.to_string(),
                logo_urls_whitelist: config.logo_urls_whitelist,
                push_update_contracts: config
                    .push_update_contracts
                    .into_iter()
                    .map(|a| a.to_string())
                    .collect(),
            })?)
        },
        QueryMsg::Balance {
            address,
        } => Ok(to_binary(&get_user_balance(deps, env, address)?)?),
        QueryMsg::TokenInfo {} => Ok(to_binary(&query_token_info(deps, env)?)?),
        QueryMsg::MarketingInfo {} => Ok(to_binary(&query_marketing_info(deps)?)?),
        QueryMsg::DownloadLogo {} => Ok(to_binary(&query_download_logo(deps)?)?),
    }
}

/// Checks if specified addresses are blacklisted.
///
/// * **voters** addresses to check if they are blacklisted.
pub fn check_voters_are_blacklisted(
    deps: Deps,
    voters: Vec<String>,
) -> Result<BlacklistedVotersResponse, ContractError> {
    let black_list = BLACKLIST.load(deps.storage)?;

    for voter in voters {
        let voter_addr = deps.api.addr_validate(voter.as_str())?;
        if !black_list.contains(&voter_addr) {
            return Ok(BlacklistedVotersResponse::VotersNotBlacklisted {
                voter,
            });
        }
    }

    Ok(BlacklistedVotersResponse::VotersBlacklisted {})
}

/// Returns a list of blacklisted voters.
///
/// * **start_after** is an optional field that specifies whether the function should return
/// a list of voters starting from a specific address onward.
///
/// * **limit** max amount of voters addresses to return.
pub fn get_blacklisted_voters(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<Addr>, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let mut black_list = BLACKLIST.load(deps.storage)?;

    if black_list.is_empty() {
        return Ok(vec![]);
    }

    black_list.sort();

    let mut start_index = Default::default();
    if let Some(start_after) = start_after {
        let start_addr = deps.api.addr_validate(start_after.as_str())?;
        start_index = black_list
            .iter()
            .position(|addr| *addr == start_addr)
            .ok_or_else(|| ContractError::AddressNotBlacklisted(start_addr.to_string()))?
            + 1; // start from the next element of the slice
    }

    // validate end index of the slice
    let end_index = (start_index + limit).min(black_list.len());

    Ok(black_list[start_index..end_index].to_vec())
}

/// Return a user's lock information.
///
/// * **user** user for which we return lock information.
fn get_user_lock_info(
    deps: Deps,
    env: &Env,
    user: String,
) -> Result<LockInfoResponse, ContractError> {
    let addr = deps.api.addr_validate(&user)?;
    if let Some(lock) = LOCKED.may_load(deps.storage, addr.clone())? {
        let cur_period = get_period(env.block.time.seconds())?;

        let last_checkpoint = fetch_last_checkpoint(deps.storage, &addr, cur_period)?;
        // The voting power point at the specified `time` was found
        let (voting_power, slope, fixed_amount) =
            if let Some(point) = last_checkpoint.map(|(_, point)| point) {
                if point.start == cur_period {
                    (point.power, point.slope, point.fixed)
                } else {
                    // The point before the intended period was found, thus we can calculate the user's voting power for the period we want
                    (calc_voting_power(&point, cur_period), point.slope, point.fixed)
                }
            } else {
                (Uint128::zero(), Uint128::zero(), Uint128::zero())
            };

        let coefficient = calc_coefficient(lock.end - lock.last_extend_lock_period);

        let resp = LockInfoResponse {
            amount: lock.amount,
            coefficient,
            start: lock.start,
            end: lock.end,
            voting_power,
            fixed_amount,
            slope,
        };
        Ok(resp)
    } else {
        Err(ContractError::UserNotFound(addr.to_string()))
    }
}

/// Return a user's staked ampLP amount at a given block height.
///
/// * **user** user for which we return lock information.
///
/// * **block_height** block height at which we return the staked ampLP amount.
fn get_user_deposit_at_height(deps: Deps, user: String, block_height: u64) -> StdResult<Uint128> {
    let addr = deps.api.addr_validate(&user)?;
    let locked_opt = LOCKED.may_load_at_height(deps.storage, addr, block_height)?;
    if let Some(lock) = locked_opt {
        Ok(lock.amount)
    } else {
        Ok(Uint128::zero())
    }
}

/// Calculates a user's voting power at a given timestamp.
/// If time is None, then it calculates the user's voting power at the current block.
///
/// * **user** user/staker for which we fetch the current voting power (vAMP balance).
///
/// * **time** timestamp at which to fetch the user's voting power (vAMP balance).
fn get_user_vamp(
    deps: Deps,
    env: Env,
    user: String,
    time: Option<u64>,
) -> StdResult<VotingPowerResponse> {
    let period = get_period(time.unwrap_or_else(|| env.block.time.seconds()))?;
    get_user_vamp_at_period(deps, user, period)
}

/// Calculates a user's voting power at a given period number.
///
/// * **user** user/staker for which we fetch the current voting power (vAMP balance).
///
/// * **period** period number at which to fetch the user's voting power (vAMP balance).
fn get_user_vamp_at_period(
    deps: Deps,
    user: String,
    period: u64,
) -> StdResult<VotingPowerResponse> {
    let user = deps.api.addr_validate(&user)?;
    let last_checkpoint = fetch_last_checkpoint(deps.storage, &user, period)?;

    if let Some(point) = last_checkpoint.map(|(_, point)| point) {
        // The voting power point at the specified `time` was found
        let voting_power = if point.start == period {
            point.power + point.fixed
        } else if point.end <= period {
            // the current period is after the voting end -> get default end power.
            point.fixed
        } else {
            // The point before the intended period was found, thus we can calculate the user's voting power for the period we want
            calc_voting_power(&point, period) + point.fixed
        };
        Ok(VotingPowerResponse {
            vamp: voting_power,
        })
    } else {
        // User not found
        Ok(VotingPowerResponse {
            vamp: Uint128::zero(),
        })
    }
}

/// Calculates a user's voting power at the current block.
///
/// * **user** user/staker for which we fetch the current voting power (vAMP balance).
fn get_user_balance(deps: Deps, env: Env, user: String) -> StdResult<BalanceResponse> {
    let vp_response = get_user_vamp(deps, env, user, None)?;
    Ok(BalanceResponse {
        balance: vp_response.vamp,
    })
}

/// Calculates the total voting power (total vAMP supply) at the given timestamp.
/// If `time` is None, then it calculates the total voting power at the current block.
///
/// * **time** timestamp at which we fetch the total voting power (vAMP supply).
fn get_total_vamp(deps: Deps, env: Env, time: Option<u64>) -> StdResult<VotingPowerResponse> {
    let period = get_period(time.unwrap_or_else(|| env.block.time.seconds()))?;
    get_total_vamp_at_period(deps, env, period)
}

/// Calculates the total voting power (total vAMP supply) at the given period number.
///
/// * **period** period number at which we fetch the total voting power (vAMP supply).
fn get_total_vamp_at_period(deps: Deps, env: Env, period: u64) -> StdResult<VotingPowerResponse> {
    let last_checkpoint = fetch_last_checkpoint(deps.storage, &env.contract.address, period)?;

    let point = last_checkpoint.map_or(
        Point {
            power: Uint128::zero(),
            start: period,
            end: period,
            slope: Default::default(),
            fixed: Uint128::zero(),
        },
        |(_, point)| point,
    );

    let voting_power = if point.start == period {
        point.power + point.fixed
    } else {
        let scheduled_slope_changes = fetch_slope_changes(deps.storage, point.start, period)?;
        let mut init_point = point;
        for (recalc_period, scheduled_change) in scheduled_slope_changes {
            init_point = Point {
                power: calc_voting_power(&init_point, recalc_period),
                start: recalc_period,
                slope: init_point.slope - scheduled_change,
                fixed: init_point.fixed,
                ..init_point
            }
        }
        calc_voting_power(&init_point, period) + init_point.fixed
    };

    Ok(VotingPowerResponse {
        vamp: voting_power,
    })
}

/// Fetch the vAMP token information, such as the token name, symbol, decimals and total supply (total voting power).
fn query_token_info(deps: Deps, env: Env) -> StdResult<TokenInfoResponse> {
    let info = TOKEN_INFO.load(deps.storage)?;
    let total_vp = get_total_vamp(deps, env, None)?;
    let res = TokenInfoResponse {
        name: info.name,
        symbol: info.symbol,
        decimals: info.decimals,
        total_supply: total_vp.vamp,
    };
    Ok(res)
}

/// Manages contract migration.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let contract_version = get_contract_version(deps.storage)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if contract_version.contract != CONTRACT_NAME {
        return Err(ContractError::MigrationError(format!(
            "contract_name does not match: prev: {0}, new: {1}",
            contract_version.contract, CONTRACT_VERSION
        )));
    }

    Ok(Response::new()
        .add_attribute("previous_contract_name", &contract_version.contract)
        .add_attribute("previous_contract_version", &contract_version.version)
        .add_attribute("new_contract_name", CONTRACT_NAME)
        .add_attribute("new_contract_version", CONTRACT_VERSION))
}
