use cosmwasm_std::{Decimal, Response, StdError, StdResult};

pub const CONTRACT_NAME: &str = "eris-yield-extractor";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_yield_extract_max() -> Decimal {
    // 100% max yield extract
    Decimal::from_ratio(1_u128, 1_u128)
}
pub fn get_yield_extract_min() -> Decimal {
    // 0% min yield extract
    Decimal::from_ratio(0_u128, 1_u128)
}

pub fn assert_valid_yield_extract(value: &Decimal) -> StdResult<Response> {
    if value.gt(&get_yield_extract_max()) {
        return Err(StdError::generic_err("'yield_extract' greater than max"));
    }

    if value.lt(&get_yield_extract_min()) {
        return Err(StdError::generic_err("'yield_extract' less than min"));
    }

    Ok(Response::new())
}
