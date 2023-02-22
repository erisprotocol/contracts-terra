use crate::error::CustomResult;
use crate::extensions::ConfigEx;
use cosmwasm_std::{Decimal, QuerierWrapper, StdResult, Uint128};
use eris::arb_vault::{ValidatedConfig, ValidatedFeeConfig};
use std::ops::Mul;

//----------------------------------------------------------------------------------------
//  HELPERS
//----------------------------------------------------------------------------------------

pub fn get_share_from_deposit(
    querier: &QuerierWrapper,
    config: &ValidatedConfig,
    total_utoken: Uint128,
    deposit_amount: Uint128,
) -> StdResult<Uint128> {
    let total_lp_supply = config.query_lp_supply(querier)?;
    let share = if total_lp_supply.is_zero() {
        // Initial share = collateral amount
        Uint128::new(deposit_amount.u128())
    } else {
        // 1. sqrt(deposit_0 * exchange_rate_0_to_1 * deposit_0) * (total_lp_supply / sqrt(pool_0 * pool_1))
        // == deposit_0 * total_lp_supply / pool_0
        deposit_amount.multiply_ratio(total_lp_supply, total_utoken)
    };
    Ok(share)
}

pub fn calc_fees(
    fee: &ValidatedFeeConfig,
    withdraw_amount: Uint128,
    withdraw_pool_fee_factor: Decimal,
) -> CustomResult<(Uint128, Uint128)> {
    let withdraw_protocol_fee = withdraw_amount * fee.protocol_withdraw_fee;

    // pool_fee_factor
    // = 1 for immediate withdraws
    // = ]0,1] for immediate withdraws after time x
    // = 0 for withdraw after unbond
    let withdraw_pool_fee =
        withdraw_amount * withdraw_pool_fee_factor.mul(fee.immediate_withdraw_fee);

    Ok((withdraw_protocol_fee, withdraw_pool_fee))
}
