use astroport::asset::AssetInfo;
use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

/// ## Description
/// This enum describes maker contract errors!
#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid bridge {0} to {1}")]
    InvalidBridge(AssetInfo, AssetInfo),

    #[error("Invalid bridge destination. {0} cannot be swapped to ASTRO")]
    InvalidBridgeDestination(AssetInfo),

    #[error("Max bridge length of {0} was reached")]
    MaxBridgeDepth(u64),

    #[error("Cannot swap {0}. No swap destinations")]
    CannotSwap(AssetInfo),

    #[error("Incorrect max spread")]
    IncorrectMaxSpread {},

    #[error("Cannot collect. Remove duplicate asset")]
    DuplicatedAsset {},
}

impl From<OverflowError> for ContractError {
    fn from(o: OverflowError) -> Self {
        StdError::from(o).into()
    }
}
