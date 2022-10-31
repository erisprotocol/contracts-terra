use cosmwasm_std::{Addr, Api, Reply, StdError, StdResult, SubMsgResponse, Uint128, Uint256};
use std::convert::TryInto;

// pub fn compute_deposit_time(
//     last_deposit_amount: Uint128,
//     new_deposit_amount: Uint128,
//     last_deposit_time: u64,
//     new_deposit_time: u64,
// ) -> StdResult<u64> {
//     let last_weight = last_deposit_amount.u128() * (last_deposit_time as u128);
//     let new_weight = new_deposit_amount.u128() * (new_deposit_time as u128);
//     let weight_avg =
//         (last_weight + new_weight) / (last_deposit_amount.u128() + new_deposit_amount.u128());
//     u64::try_from(weight_avg).map_err(|_| StdError::generic_err("Overflow in compute_deposit_time"))
// }

/// Unwrap a `Reply` object to extract the response
pub fn unwrap_reply(reply: Reply) -> StdResult<SubMsgResponse> {
    reply.result.into_result().map_err(StdError::generic_err)
}

pub trait ScalingUint128 {
    fn multiply_ratio_and_ceil(&self, numerator: Uint128, denominator: Uint128) -> Uint128;
}

/// Returns a lowercased, validated address upon success. Otherwise returns [`Err`]
/// ## Params
/// * **api** is an object of type [`Api`]
///
/// * **addr** is an object of type [`Addr`]
pub fn addr_validate_to_lower(api: &dyn Api, addr: impl Into<String>) -> StdResult<Addr> {
    let addr = addr.into();
    if addr.to_lowercase() != addr {
        return Err(StdError::generic_err(format!("Address {} should be lowercase", addr)));
    }
    api.addr_validate(&addr)
}

/// Returns a lowercased, validated address upon success if present.
pub fn addr_opt_validate(api: &dyn Api, addr: &Option<String>) -> StdResult<Option<Addr>> {
    addr.as_ref().map(|addr| addr_validate_to_lower(api, addr)).transpose()
}

impl ScalingUint128 for Uint128 {
    /// Multiply Uint128 by Decimal, rounding up to the nearest integer.
    fn multiply_ratio_and_ceil(
        self: &Uint128,
        numerator: Uint128,
        denominator: Uint128,
    ) -> Uint128 {
        let x = self.full_mul(numerator);
        let y: Uint256 = denominator.into();
        ((x + y - Uint256::from(1u64)) / y).try_into().expect("multiplication overflow")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiply_ratio_and_ceil() {
        let a = Uint128::new(124);
        let b = a.multiply_ratio_and_ceil(Uint128::new(1), Uint128::new(3));
        assert_eq!(b, Uint128::new(42));

        let a = Uint128::new(123);
        let b = a.multiply_ratio_and_ceil(Uint128::new(1), Uint128::new(3));
        assert_eq!(b, Uint128::new(41));
    }
}
