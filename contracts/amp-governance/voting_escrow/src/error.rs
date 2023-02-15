use cosmwasm_std::{OverflowError, StdError};
use cw20_base::ContractError as cw20baseError;
use thiserror::Error;

/// This enum describes vAMP contract errors
#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Cw20Base(#[from] cw20baseError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{location:?}: {orig:?}")]
    OverflowLocation {
        location: String,
        orig: OverflowError,
    },

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Lock already exists")]
    LockAlreadyExists {},

    #[error("Lock does not exist")]
    LockDoesNotExist {},

    #[error("User {0} not found")]
    UserNotFound(String),

    #[error("Lock time must be within limits (week <= lock time < 2 years)")]
    LockTimeLimitsError {},

    #[error("Lock period must be 3 or more weeks")]
    LockPeriodsError {},

    #[error("The lock time has not yet expired")]
    LockHasNotExpired {},

    #[error("The lock expired. Withdraw and create new lock")]
    LockExpired {},

    #[error("The {0} address is blacklisted")]
    AddressBlacklisted(String),

    #[error("The {0} address is not blacklisted")]
    AddressNotBlacklisted(String),

    #[error("Do not send the address {0} multiple times. (Blacklist)")]
    AddressBlacklistDuplicated(String),

    #[error("Append and remove arrays are empty")]
    AddressBlacklistEmpty {},

    #[error("Marketing info validation error: {0}")]
    MarketingInfoValidationError(String),

    #[error("Logo binary data exceeds 5KB limit")]
    LogoTooBig {},

    #[error("Invalid xml preamble for SVG")]
    InvalidXmlPreamble {},

    #[error("Invalid png header")]
    InvalidPngHeader {},

    #[error("Checkpoint initialization error")]
    CheckpointInitializationFailed {},

    #[error("Contract can't be migrated: {0}")]
    MigrationError(String),
}
