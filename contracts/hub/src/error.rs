use cosmwasm_std::{OverflowError, Response, StdError};
use cw20_base::ContractError as cw20baseError;
use thiserror::Error;

pub type ContractResult = Result<Response, ContractError>;

/// This enum describes hub contract errors
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Cw20Base(#[from] cw20baseError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("Unauthorized: sender is not owner")]
    Unauthorized {},

    #[error("Unauthorized: sender is not new owner")]
    UnauthorizedSenderNotNewOwner {},

    #[error("Unauthorized: sender is not vote operator")]
    UnauthorizedSenderNotVoteOperator {},

    #[error("Expecting stake token, received {0}")]
    ExpectingStakeToken(String),

    #[error("Protocol_reward_fee greater than max")]
    ProtocolRewardFeeTooHigh {},

    #[error("{0} can't be zero")]
    CantBeZero(String),

    #[error("Batch can only be submitted for unbonding after {0}")]
    SubmitBatchAfter(u64),

    #[error("Callbacks can only be invoked by the contract itself")]
    CallbackOnlyCalledByContract {},

    #[error("Invalid reply id: {0}")]
    InvalidReplyId(u64),

    #[error("Donations are disabled")]
    DonationsDisabled {},

    #[error("No {0} available to be bonded")]
    NoTokensAvailable(String),

    #[error("validator {0} is already whitelisted")]
    ValidatorAlreadyWhitelisted(String),

    #[error("validator {0} is not whitelisted")]
    ValidatorNotWhitelisted(String),

    #[error("cannot find `instantiate` event")]
    CannotFindInstantiateEvent {},

    #[error("cannot find `_contract_address` attribute")]
    CannotFindContractAddress {},

    #[error("No vote operator set")]
    NoVoteOperatorSet {},

    #[error("Contract can't be migrated!")]
    MigrationError {},
}
