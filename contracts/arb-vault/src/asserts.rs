use crate::error::ContractError;
use crate::extensions::BalancesEx;

use cosmwasm_std::{Decimal, Uint128};
use eris::arb_vault::{BalancesDetails, ValidatedConfig};

//----------------------------------------------------------------------------------------
//  ASSERTS
//----------------------------------------------------------------------------------------

pub fn assert_max_amount(
    config: &ValidatedConfig,
    balances: &BalancesDetails,
    wanted_profit: &Decimal,
    wanted_amount: &Uint128,
) -> Result<(), ContractError> {
    let takeable = balances.calc_takeable_for_profit(config, wanted_profit)?;
    if takeable.lt(wanted_amount) {
        return Err(ContractError::NotEnoughFundsTakeable {});
    }

    Ok(())
}

pub fn assert_min_profit(wanted_profit: &Decimal) -> Result<(), ContractError> {
    // min profit must be bigger than 0.5 % (5/1000)
    if wanted_profit.lt(&Decimal::from_ratio(5u128, 1000u128)) {
        return Err(ContractError::NotEnoughProfit {});
    }

    Ok(())
}

pub fn assert_has_funds(funds_amount: &Uint128) -> Result<(), ContractError> {
    if funds_amount.is_zero() {
        return Err(ContractError::InvalidZeroAmount {});
    }

    Ok(())
}
