use crate::error::ContractError;
use eris::governance_helper::{MAX_LOCK_TIME, MIN_LOCK_PERIODS, WEEK};

use cosmwasm_std::{Addr, Order, StdResult, Storage, Uint128};
use cw_storage_plus::Bound;

use crate::state::{Point, BLACKLIST, CONFIG, HISTORY, LAST_SLOPE_CHANGE, SLOPE_CHANGES};

/// Checks that a timestamp is within limits.
pub(crate) fn assert_time_limits(time: u64) -> Result<(), ContractError> {
    if !(WEEK..=MAX_LOCK_TIME).contains(&time) {
        Err(ContractError::LockTimeLimitsError {})
    } else {
        Ok(())
    }
}

pub(crate) fn assert_periods_remaining(periods: u64) -> Result<(), ContractError> {
    if periods < MIN_LOCK_PERIODS {
        Err(ContractError::LockPeriodsError {})
    } else {
        Ok(())
    }
}

/// Checks that the sender is the ampLP token.
pub(crate) fn assert_token(storage: &dyn Storage, sender: Addr) -> Result<(), ContractError> {
    let config = CONFIG.load(storage)?;
    if sender != config.deposit_token_addr {
        Err(ContractError::Unauthorized {})
    } else {
        Ok(())
    }
}

/// Checks if the blacklist contains a specific address.
pub(crate) fn assert_blacklist(storage: &dyn Storage, addr: &Addr) -> Result<(), ContractError> {
    let blacklist = BLACKLIST.load(storage)?;
    if blacklist.contains(addr) {
        Err(ContractError::AddressBlacklisted(addr.to_string()))
    } else {
        Ok(())
    }
}

/// Main function used to calculate a user's voting power at a specific period as: previous_power - slope*(x - previous_x).
pub(crate) fn calc_voting_power(point: &Point, period: u64) -> Uint128 {
    let shift = point
        .slope
        .checked_mul(Uint128::from(period - point.start))
        .unwrap_or_else(|_| Uint128::zero());
    point.power.checked_sub(shift).unwrap_or_else(|_| Uint128::zero())
}

/// Fetches the last checkpoint in [`HISTORY`] for the given address.
pub(crate) fn fetch_last_checkpoint(
    storage: &dyn Storage,
    addr: &Addr,
    period_key: u64,
) -> StdResult<Option<(u64, Point)>> {
    HISTORY
        .prefix(addr.clone())
        .range(storage, None, Some(Bound::inclusive(period_key)), Order::Descending)
        .next()
        .transpose()
}

/// Cancels scheduled slope change of total voting power only if the given period is in future.
/// Removes scheduled slope change if it became zero.
pub(crate) fn cancel_scheduled_slope(
    storage: &mut dyn Storage,
    slope: Uint128,
    period: u64,
) -> StdResult<u64> {
    let end_period_key = period;
    let last_slope_change = LAST_SLOPE_CHANGE.may_load(storage)?.unwrap_or(0);

    // We do not need to schedule a slope change in the past
    if period > last_slope_change {
        match SLOPE_CHANGES.may_load(storage, end_period_key)? {
            Some(old_scheduled_change) => {
                let new_slope = old_scheduled_change.saturating_sub(slope);
                if !new_slope.is_zero() {
                    SLOPE_CHANGES.save(storage, end_period_key, &(old_scheduled_change - slope))?;
                } else {
                    SLOPE_CHANGES.remove(storage, end_period_key);
                }

                Ok(last_slope_change)
            },
            _ => Ok(last_slope_change),
        }
    } else {
        Ok(last_slope_change)
    }
}

/// Schedules slope change of total voting power in the given period.
pub(crate) fn schedule_slope_change(
    storage: &mut dyn Storage,
    slope: Uint128,
    period: u64,
) -> StdResult<()> {
    if !slope.is_zero() {
        SLOPE_CHANGES
            .update(storage, period, |slope_opt| -> StdResult<Uint128> {
                if let Some(pslope) = slope_opt {
                    Ok(pslope + slope)
                } else {
                    Ok(slope)
                }
            })
            .map(|_| ())
    } else {
        Ok(())
    }
}

/// Fetches all slope changes between `last_slope_change` and `period`.
pub(crate) fn fetch_slope_changes(
    storage: &dyn Storage,
    last_slope_change: u64,
    period: u64,
) -> StdResult<Vec<(u64, Uint128)>> {
    SLOPE_CHANGES
        .range(
            storage,
            Some(Bound::exclusive(last_slope_change)),
            Some(Bound::inclusive(period)),
            Order::Ascending,
        )
        .collect()
}
