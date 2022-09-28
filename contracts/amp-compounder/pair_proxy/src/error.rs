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

    #[error("Invalid asset")]
    InvalidAsset {},

    #[error("Duplicated assets in asset infos")]
    DuplicatedAssets {},

    #[error("Must provide at least 2 assets!")]
    MustProvideNAssets {},

    #[error("The limit exceeded of swap assets!")]
    SwapLimitExceeded {},

}

impl From<OverflowError> for ContractError {
    fn from(o: OverflowError) -> Self {
        StdError::from(o).into()
    }
}
