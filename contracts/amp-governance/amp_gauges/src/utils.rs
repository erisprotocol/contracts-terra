use std::ops::RangeInclusive;

use cosmwasm_std::{Addr, Order, QuerierWrapper, StdError, StdResult, Storage, Uint128};
use cw_storage_plus::Bound;

use eris::governance_helper::calc_voting_power;
use eris::helpers::bps::BasicPoints;
use eris::hub::get_hub_validators;

use crate::error::ContractError;
use crate::state::{
    VotedValidatorInfo, VALIDATORS, VALIDATOR_FIXED_VAMP, VALIDATOR_PERIODS,
    VALIDATOR_SLOPE_CHANGES, VALIDATOR_VOTES,
};

/// Pools limit should be within the range `[2, 100]`
const VALIDATOR_NUMBER_LIMIT: RangeInclusive<u64> = 2..=100;

/// The enum defines math operations with voting power and slope.
#[derive(Debug)]
pub(crate) enum Operation {
    Add,
    Sub,
}

impl Operation {
    pub fn calc_slope(&self, cur_slope: Uint128, slope: Uint128, bps: BasicPoints) -> Uint128 {
        match self {
            Operation::Add => cur_slope + bps * slope,
            Operation::Sub => cur_slope - bps * slope,
        }
    }

    pub fn calc_voting_power(&self, cur_vp: Uint128, vp: Uint128, bps: BasicPoints) -> Uint128 {
        match self {
            Operation::Add => cur_vp + bps * vp,
            Operation::Sub => cur_vp.saturating_sub(bps * vp),
        }
    }
}

/// Enum wraps [`VotedPoolInfo`] so the contract can leverage storage operations efficiently.
#[derive(Debug)]
pub(crate) enum VotedPoolInfoResult {
    Unchanged(VotedValidatorInfo),
    New(VotedValidatorInfo),
}

/// Filters pairs (LP token address, voting parameters) by criteria:
/// * pool's pair is registered in Factory,
/// * pool's pair type is not in blocked list,
/// * any of pair's token is not listed in blocked tokens list.
pub(crate) fn filter_validators(
    querier: &QuerierWrapper,
    hub_addr: &Addr,
    validators: Vec<(Addr, Uint128)>,
    validators_limit: u64,
) -> StdResult<Vec<(Addr, Uint128)>> {
    let allowed_validators = get_hub_validators(querier, hub_addr)?;

    let validators = validators
        .into_iter()
        .filter_map(|(validator_addr, vxastro_amount)| {
            if allowed_validators.contains(&validator_addr.to_string()) {
                Some((validator_addr, vxastro_amount))
            } else {
                None
            }
        })
        .take(validators_limit as usize)
        .collect();

    Ok(validators)
}

/// Cancels user changes using old voting parameters for a given pool.  
/// Firstly, it removes slope change scheduled for previous lockup end period.  
/// Secondly, it updates voting parameters for the given period, but without user's vote.
pub(crate) fn cancel_user_changes(
    storage: &mut dyn Storage,
    period: u64,
    validator_addr: &Addr,
    old_bps: BasicPoints,
    old_vp: Uint128,
    old_slope: Uint128,
    old_lock_end: u64,
) -> StdResult<()> {
    // Cancel scheduled slope changes
    let last_validator_period =
        fetch_last_validator_period(storage, period, validator_addr)?.unwrap_or(period);
    if last_validator_period < old_lock_end + 1 {
        let end_period_key = old_lock_end + 1;
        let old_scheduled_change =
            VALIDATOR_SLOPE_CHANGES.load(storage, (validator_addr, end_period_key))?;
        let new_slope = old_scheduled_change - old_bps * old_slope;
        if !new_slope.is_zero() {
            VALIDATOR_SLOPE_CHANGES.save(storage, (validator_addr, end_period_key), &new_slope)?
        } else {
            VALIDATOR_SLOPE_CHANGES.remove(storage, (validator_addr, end_period_key))
        }
    }

    update_validator_info(
        storage,
        period,
        validator_addr,
        Some((old_bps, old_vp, old_slope, Operation::Sub)),
    )
    .map(|_| ())
}

/// Applies user's vote for a given pool.   
/// Firstly, it schedules slope change for lockup end period.  
/// Secondly, it updates voting parameters with applied user's vote.
pub(crate) fn vote_for_validator(
    storage: &mut dyn Storage,
    period: u64,
    validator_addr: &Addr,
    bps: BasicPoints,
    vp: Uint128,
    slope: Uint128,
    lock_end: u64,
) -> StdResult<()> {
    // Schedule slope changes
    VALIDATOR_SLOPE_CHANGES.update::<_, StdError>(
        storage,
        (validator_addr, lock_end + 1),
        |slope_opt| {
            if let Some(saved_slope) = slope_opt {
                Ok(saved_slope + bps * slope)
            } else {
                Ok(bps * slope)
            }
        },
    )?;
    update_validator_info(storage, period, validator_addr, Some((bps, vp, slope, Operation::Add)))
        .map(|_| ())
}

pub(crate) fn add_fixed_vamp(
    storage: &mut dyn Storage,
    period: u64,
    validator_addr: &Addr,
    vamps: Uint128,
) -> StdResult<()> {
    add_validator_to_active(storage, validator_addr)?;

    // always change the future period only
    let period = period + 1;
    let last = fetch_last_validator_fixed_vamp_value(storage, period, validator_addr)?;
    let new = last.checked_add(vamps)?;
    VALIDATOR_FIXED_VAMP.save(storage, (validator_addr, period), &new)?;

    Ok(())
}

pub(crate) fn remove_fixed_vamp(
    storage: &mut dyn Storage,
    period: u64,
    validator_addr: &Addr,
    vamps: Uint128,
) -> StdResult<()> {
    add_validator_to_active(storage, validator_addr)?;

    // always change the future period only
    let period = period + 1;
    let last = fetch_last_validator_fixed_vamp_value(storage, period, validator_addr)?;
    let new = last.checked_sub(vamps)?;
    VALIDATOR_FIXED_VAMP.save(storage, (validator_addr, period), &new)?;

    Ok(())
}

/// Fetches voting parameters for a given pool at specific period, applies new changes, saves it in storage
/// and returns new voting parameters in [`VotedPoolInfo`] object.
/// If there are no changes in 'changes' parameter
/// and voting parameters were already calculated before the function just returns [`VotedPoolInfo`].
pub(crate) fn update_validator_info(
    storage: &mut dyn Storage,
    period: u64,
    validator_addr: &Addr,
    changes: Option<(BasicPoints, Uint128, Uint128, Operation)>,
) -> StdResult<VotedValidatorInfo> {
    add_validator_to_active(storage, validator_addr)?;
    let period_key = period;
    let validator_info = match get_validator_info_mut(storage, period, validator_addr)? {
        VotedPoolInfoResult::Unchanged(mut validator_info)
        | VotedPoolInfoResult::New(mut validator_info)
            if changes.is_some() =>
        {
            if let Some((bps, vp, slope, op)) = changes {
                validator_info.slope = op.calc_slope(validator_info.slope, slope, bps);
                validator_info.vamp_amount =
                    op.calc_voting_power(validator_info.vamp_amount, vp, bps);
            }
            VALIDATOR_PERIODS.save(storage, (validator_addr, period_key), &())?;
            VALIDATOR_VOTES.save(storage, (period_key, validator_addr), &validator_info)?;
            validator_info
        },
        VotedPoolInfoResult::New(validator_info) => {
            VALIDATOR_PERIODS.save(storage, (validator_addr, period_key), &())?;
            VALIDATOR_VOTES.save(storage, (period_key, validator_addr), &validator_info)?;
            validator_info
        },
        VotedPoolInfoResult::Unchanged(validator_info) => validator_info,
    };

    Ok(validator_info)
}

fn add_validator_to_active(
    storage: &mut dyn Storage,
    validator_addr: &Addr,
) -> Result<(), StdError> {
    if VALIDATORS.may_load(storage, validator_addr)?.is_none() {
        VALIDATORS.save(storage, validator_addr, &())?
    };
    Ok(())
}

/// Returns pool info at specified period or calculates it. Saves intermediate results in storage.
pub(crate) fn get_validator_info_mut(
    storage: &mut dyn Storage,
    period: u64,
    validator_addr: &Addr,
) -> StdResult<VotedPoolInfoResult> {
    let validator_info_result = if let Some(validator_info) =
        VALIDATOR_VOTES.may_load(storage, (period, validator_addr))?
    {
        VotedPoolInfoResult::Unchanged(validator_info)
    } else {
        let validator_info_result = if let Some(mut prev_period) =
            fetch_last_validator_period(storage, period, validator_addr)?
        {
            let mut validator_info =
                VALIDATOR_VOTES.load(storage, (prev_period, validator_addr))?;
            // Recalculating passed periods
            let scheduled_slope_changes =
                fetch_slope_changes(storage, validator_addr, prev_period, period)?;
            for (recalc_period, scheduled_change) in scheduled_slope_changes {
                validator_info = VotedValidatorInfo {
                    vamp_amount: calc_voting_power(
                        validator_info.slope,
                        validator_info.vamp_amount,
                        prev_period,
                        recalc_period,
                    ),
                    slope: validator_info.slope - scheduled_change,
                };
                // Save intermediate result
                let recalc_period_key = recalc_period;
                VALIDATOR_PERIODS.save(storage, (validator_addr, recalc_period_key), &())?;
                VALIDATOR_VOTES.save(
                    storage,
                    (recalc_period_key, validator_addr),
                    &validator_info,
                )?;
                prev_period = recalc_period
            }

            VotedValidatorInfo {
                vamp_amount: calc_voting_power(
                    validator_info.slope,
                    validator_info.vamp_amount,
                    prev_period,
                    period,
                ),
                ..validator_info
            }
        } else {
            VotedValidatorInfo::default()
        };

        VotedPoolInfoResult::New(validator_info_result)
    };

    Ok(validator_info_result)
}

/// Returns pool info at specified period or calculates it.
pub(crate) fn get_validator_info(
    storage: &dyn Storage,
    period: u64,
    validator_addr: &Addr,
) -> StdResult<VotedValidatorInfo> {
    let fixed_amount = fetch_last_validator_fixed_vamp_value(storage, period, validator_addr)?;

    let validator_info = if let Some(validator_info) =
        VALIDATOR_VOTES.may_load(storage, (period, validator_addr))?
    {
        VotedValidatorInfo {
            vamp_amount: validator_info.vamp_amount.checked_add(fixed_amount)?,
            slope: validator_info.slope,
        }
    } else if let Some(mut prev_period) =
        fetch_last_validator_period(storage, period, validator_addr)?
    {
        let mut validator_info = VALIDATOR_VOTES.load(storage, (prev_period, validator_addr))?;
        // Recalculating passed periods
        let scheduled_slope_changes =
            fetch_slope_changes(storage, validator_addr, prev_period, period)?;
        for (recalc_period, scheduled_change) in scheduled_slope_changes {
            validator_info = VotedValidatorInfo {
                vamp_amount: calc_voting_power(
                    validator_info.slope,
                    validator_info.vamp_amount,
                    prev_period,
                    recalc_period,
                ),
                slope: validator_info.slope - scheduled_change,
            };
            prev_period = recalc_period
        }

        VotedValidatorInfo {
            vamp_amount: calc_voting_power(
                validator_info.slope,
                validator_info.vamp_amount,
                prev_period,
                period,
            )
            .checked_add(fixed_amount)?,
            ..validator_info
        }
    } else if !fixed_amount.is_zero() {
        VotedValidatorInfo {
            vamp_amount: fixed_amount,
            slope: Uint128::zero(),
        }
    } else {
        VotedValidatorInfo::default()
    };

    Ok(validator_info)
}

/// Fetches last period for specified pool which has saved result in [`VALIDATOR_PERIODS`].
pub(crate) fn fetch_last_validator_period(
    storage: &dyn Storage,
    period: u64,
    validator_addr: &Addr,
) -> StdResult<Option<u64>> {
    let period_opt = VALIDATOR_PERIODS
        .prefix(validator_addr)
        .range(storage, None, Some(Bound::exclusive(period)), Order::Descending)
        .next()
        .transpose()?
        .map(|(period, _)| period);
    Ok(period_opt)
}

pub(crate) fn fetch_last_validator_fixed_vamp_value(
    storage: &dyn Storage,
    period: u64,
    validator_addr: &Addr,
) -> StdResult<Uint128> {
    let result = fetch_last_validator_fixed_vamp(storage, period, validator_addr)?;
    Ok(result.unwrap_or_default())
}

pub(crate) fn fetch_last_validator_fixed_vamp(
    storage: &dyn Storage,
    period: u64,
    validator_addr: &Addr,
) -> StdResult<Option<Uint128>> {
    let emps_opt = VALIDATOR_FIXED_VAMP
        .prefix(validator_addr)
        .range(storage, None, Some(Bound::inclusive(period)), Order::Descending)
        .next()
        .transpose()?
        .map(|(_, emps)| emps);
    Ok(emps_opt)
}

/// Fetches all slope changes between `last_period` and `period` for specific pool.
pub(crate) fn fetch_slope_changes(
    storage: &dyn Storage,
    validator_addr: &Addr,
    last_period: u64,
    period: u64,
) -> StdResult<Vec<(u64, Uint128)>> {
    VALIDATOR_SLOPE_CHANGES
        .prefix(validator_addr)
        .range(
            storage,
            Some(Bound::exclusive(last_period)),
            Some(Bound::inclusive(period)),
            Order::Ascending,
        )
        .collect()
}

/// Input validation for pools limit.
pub(crate) fn validate_validators_limit(number: u64) -> Result<u64, ContractError> {
    if !VALIDATOR_NUMBER_LIMIT.contains(&number) {
        Err(ContractError::InvalidPoolNumber(number))
    } else {
        Ok(number)
    }
}
