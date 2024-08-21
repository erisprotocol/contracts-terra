use cosmwasm_std::Uint128;

//--------------------------------------------------------------------------------------------------
// Minting/burning logics
//--------------------------------------------------------------------------------------------------

pub(crate) fn compute_mint_amount(
    lp_supply: Uint128,
    stake_deposited: Uint128,
    stake_available: Uint128,
) -> Uint128 {
    if stake_available.is_zero() {
        stake_deposited
    } else {
        stake_deposited.multiply_ratio(lp_supply, stake_available)
    }
}

pub(crate) fn compute_withdraw_amount(
    lp_supply: Uint128,
    lp_to_burn: Uint128,
    stake_available: Uint128,
) -> Uint128 {
    if lp_supply.is_zero() {
        Uint128::zero()
    } else {
        lp_to_burn.multiply_ratio(stake_available, lp_supply)
    }
}
