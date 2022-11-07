use cosmwasm_std::{Decimal, StdResult, Uint128};

use crate::governance_helper::{get_periods_count, MAX_LOCK_TIME};

/// Coefficient calculation where 0 [`WEEK`] is equal to 1 and [`MAX_LOCK_TIME`] is 9.
pub fn calc_coefficient(interval: u64) -> Decimal {
    // coefficient = 9 * (end - start) / MAX_LOCK_TIME 15_u64
    // 1 is fixed
    Decimal::from_ratio(90_u64 * interval, get_periods_count(MAX_LOCK_TIME) * 10)
}

/// Adjusting voting power according to the slope. The maximum loss is 103/104 * 104 which is
/// 0.000103 vxASTRO.
pub fn adjust_vp_and_slope(vp: &mut Uint128, dt: u64) -> StdResult<Uint128> {
    let slope = vp.checked_div(Uint128::from(dt))?;
    *vp = slope * Uint128::from(dt);
    Ok(slope)
}

// pub fn adjust_vp_and_slope(vp: &mut Uint128, dt: u64, end_vp: Uint128) -> StdResult<Uint128> {
//     let slope = vp.checked_sub(end_vp)?.checked_div(Uint128::from(dt))?;
//     *vp = slope * Uint128::from(dt) + end_vp;
//     Ok(slope)
// }
