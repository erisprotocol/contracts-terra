use thiserror::Error;

use cosmwasm_std::{OverflowError, StdError, Uint128};

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Callbacks cannot be invoked externally")]
    CallbackUnauthorized {},

    #[error("Exceed quota, remaining quota is {0}")]
    ExceedQuota(Uint128),

    #[error("Cannot update {0} after set")]
    CannotUpdateAfterSet(String),
}
