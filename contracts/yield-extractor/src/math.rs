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
        lp_supply.multiply_ratio(stake_deposited, stake_available)
    }
}

pub(crate) fn compute_withdraw_amount(
    lp_supply: Uint128,
    lp_to_burn: Uint128,
    stake_available: Uint128,
) -> Uint128 {
    stake_available.multiply_ratio(lp_to_burn, lp_supply)
}
