use astroport::asset::Asset;
use cosmwasm_std::{
    Addr, Api, Coin, CosmosMsg, Env, MessageInfo, Reply, StdError, StdResult, SubMsgResponse,
    Uint128, Uint256,
};
use cw20::Expiration;
use std::convert::TryInto;

use crate::adapters::asset::AssetEx;

/// Unwrap a `Reply` object to extract the response
pub fn unwrap_reply(reply: Reply) -> StdResult<SubMsgResponse> {
    reply.result.into_result().map_err(StdError::generic_err)
}

pub trait ScalingUint128 {
    fn multiply_ratio_and_ceil(&self, numerator: Uint128, denominator: Uint128) -> Uint128;
}

/// Returns a lowercased, validated address upon success if present.
pub fn addr_opt_validate(api: &dyn Api, addr: &Option<String>) -> StdResult<Option<Addr>> {
    addr.as_ref().map(|addr| api.addr_validate(addr)).transpose()
}

/// Bulk validation and conversion between [`String`] -> [`Addr`] for an array of addresses.
/// If any address is invalid, the function returns [`StdError`].
pub fn validate_addresses(api: &dyn Api, admins: &[String]) -> StdResult<Vec<Addr>> {
    admins.iter().map(|addr| api.addr_validate(addr)).collect()
}

pub fn funds_or_allowance(
    env: &Env,
    spender: &Addr,
    assets: &[Asset],
    deposit_info: Option<&MessageInfo>,
) -> StdResult<(Vec<Coin>, Vec<CosmosMsg>)> {
    let mut funds: Vec<Coin> = vec![];
    let mut msgs: Vec<CosmosMsg> = vec![];

    for asset in assets.iter() {
        if let Some(deposit_info) = deposit_info {
            asset.deposit_asset(deposit_info, &env.contract.address, &mut msgs)?;
        }

        if !asset.amount.is_zero() {
            if asset.is_native_token() {
                funds.push(cosmwasm_std::Coin {
                    denom: asset.info.to_string(),
                    amount: asset.amount,
                });
            } else {
                msgs.push(asset.increase_allowance_msg(
                    spender.to_string(),
                    Some(Expiration::AtHeight(env.block.height + 1)),
                )?);
            }
        }
    }

    Ok((funds, msgs))
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
