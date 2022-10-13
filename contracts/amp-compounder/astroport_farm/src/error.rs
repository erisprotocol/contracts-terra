use cosmwasm_std::{OverflowError, StdError, Uint128};
use thiserror::Error;

/// ## Description
/// This enum describes pair contract errors!
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid message")]
    InvalidMessage {},

    #[error("Cannot unbond more than balance")]
    UnbondExceedBalance {},

    #[error("Assertion failed; minimum receive amount: {minimum_receive}, actual amount: {amount}")]
    AssertionMinimumReceive { minimum_receive: Uint128, amount: Uint128 },
}

impl From<OverflowError> for ContractError {
    fn from(o: OverflowError) -> Self {
        StdError::from(o).into()
    }
}
