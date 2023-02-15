use cosmwasm_std::StdError;
use thiserror::Error;

/// This enum describes contract errors
#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("You can't vote with zero voting power")]
    ZeroVotingPower {},

    #[error("Invalid validator address: {0}")]
    InvalidValidatorAddress(String),

    #[error("Votes contain duplicated validator addresses")]
    DuplicatedValidators {},

    #[error("There are no validators to tune")]
    TuneNoValidators {},

    #[error("Contract can't be migrated!")]
    MigrationError {},
}
