use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

/// ## Description
/// This enum describes pair contract errors!
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("The pair contract addr {0} can't be added multiple times.")]
    AddPairContractDuplicated(String),

    #[error("The wanted token {0} is not an asset of the pair")]
    WantedTokenNotInPair(String),

    #[error("The slippage tolarance must be less than or equal 50%")]
    SlippageTolaranaceTooHigh,
}

impl From<OverflowError> for ContractError {
    fn from(o: OverflowError) -> Self {
        StdError::from(o).into()
    }
}
