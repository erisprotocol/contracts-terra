use cosmwasm_std::{Decimal, StdResult, Uint128};

use crate::governance_helper::{get_periods_count, MAX_LOCK_TIME};

/// Coefficient calculation where 0 [`WEEK`] is equal to 2 and [`MAX_LOCK_TIME`] is 10. And trending towards 1 (end) and keeping 1.
pub fn calc_coefficient(interval: u64) -> Decimal {
    // old: coefficient = 2 + 1.5 * (end - start) / MAX_LOCK_TIME 15_u64
    // coefficient = 2 + 8 * (end - start) / MAX_LOCK_TIME 15_u64
    Decimal::from_ratio(2u64, 1u64)
        + Decimal::from_ratio(80_u64 * interval, get_periods_count(MAX_LOCK_TIME) * 10)
}

/// Adjusting voting power according to the slope. The maximum loss is 103/104 * 104 which is
/// 0.000103 vxASTRO.
pub fn adjust_vp_and_slope(vp: &mut Uint128, dt: u64, end_vp: Uint128) -> StdResult<Uint128> {
    let slope = vp.checked_sub(end_vp)?.checked_div(Uint128::from(dt))?;
    *vp = slope * Uint128::from(dt) + end_vp;
    Ok(slope)
}
