use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, QuerierWrapper, StdError, StdResult, Uint128};

use crate::voting_escrow::LockInfoResponse;

/// The maximum amount of voters that can be kicked at once from
pub const VOTERS_MAX_LIMIT: u32 = 30;

/// This structure describes the basic settings for creating a contract.
#[cw_serde]
pub struct InstantiateMsg {
    /// Contract owner
    pub owner: String,
    /// The vAMP token contract address
    pub escrow_addr: String,
    /// Hub contract address
    pub hub_addr: String,
    /// Max number of validators that can receive ASTRO emissions at the same time
    pub validators_limit: u64,
}

/// This structure describes the execute messages available in the contract.
#[cw_serde]
pub enum ExecuteMsg {
    /// Vote allows a vAMP holder to cast votes on which validators should get the delegations
    Vote {
        votes: Vec<(String, u16)>,
    },

    /// Updates the vote for a specified user. Only can be called from the escrow_addr
    UpdateVote {
        user: String,
        lock_info: LockInfoResponse,
    },

    /// TunePools transforms the latest vote distribution into alloc_points which are then applied to ASTRO generators
    TuneVamp {},
    UpdateConfig {
        /// ChangeValidatorsLimit changes the max amount of validators that can be voted at once to receive delegations
        validators_limit: Option<u64>,
    },
    // Admin action to remove a user
    RemoveUser {
        user: String,
    },
    /// ProposeNewOwner proposes a new owner for the contract
    ProposeNewOwner {
        /// Newly proposed contract owner
        new_owner: String,
        /// The timestamp when the contract ownership change expires
        expires_in: u64,
    },
    /// DropOwnershipProposal removes the latest contract ownership transfer proposal
    DropOwnershipProposal {},
    /// ClaimOwnership allows the newly proposed owner to claim contract ownership
    ClaimOwnership {},
}

/// This structure describes the query messages available in the contract.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// UserInfo returns information about a voter and the validators they voted for
    #[returns(UserInfoResponse)]
    UserInfo {
        user: String,
    },
    #[returns(UserInfosResponse)]
    UserInfos {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// TuneInfo returns information about the latest generators that were voted to receive ASTRO emissions
    #[returns(GaugeInfoResponse)]
    TuneInfo {},
    /// Config returns the contract configuration
    #[returns(ConfigResponse)]
    Config {},
    /// PoolInfo returns the latest voting power allocated to a specific pool (generator)
    #[returns(VotedValidatorInfoResponse)]
    ValidatorInfo {
        validator_addr: String,
    },
    /// PoolInfo returns the voting power allocated to a specific pool (generator) at a specific period
    #[returns(VotedValidatorInfoResponse)]
    ValidatorInfoAtPeriod {
        validator_addr: String,
        period: u64,
    },
    /// ValidatorInfos returns the latest EMPs allocated to all active validators
    #[returns(Vec<(String,VotedValidatorInfoResponse)>)]
    ValidatorInfos {
        validator_addrs: Option<Vec<String>>,
        period: Option<u64>,
    },
}

/// This structure describes a migration message.
/// We currently take no arguments for migrations.
#[cw_serde]
pub struct MigrateMsg {}

/// This structure describes the parameters returned when querying for the contract configuration.
#[cw_serde]
pub struct ConfigResponse {
    /// Address that's allowed to change contract parameters
    pub owner: Addr,
    /// The vAMP token contract address
    pub escrow_addr: Addr,
    /// Hub contract address
    pub hub_addr: Addr,
    /// Max number of validators that can receive delegations at the same time
    pub validators_limit: u64,
}

impl ConfigResponse {
    pub fn assert_owner(&self, addr: &Addr) -> StdResult<()> {
        if *addr != self.owner {
            return Err(StdError::generic_err("unauthorized"));
        }
        Ok(())
    }

    pub fn assert_owner_or_self(&self, addr: &Addr, contract_addr: &Addr) -> StdResult<()> {
        if *addr != self.owner && *addr != *contract_addr {
            return Err(StdError::generic_err("unauthorized"));
        }
        Ok(())
    }
}
/// This structure describes the response used to return voting information for a specific pool (generator).
#[cw_serde]
#[derive(Default)]
pub struct VotedValidatorInfoResponse {
    /// Dynamic voting power that voted for this validator
    pub voting_power: Uint128,
    /// fixed amount available
    pub fixed_amount: Uint128,
    /// The slope at which the amount of vAMP that voted for this validator will decay
    pub slope: Uint128,
}

/// This structure describes the response used to return tuning parameters for all pools/generators.
#[cw_serde]
#[derive(Default)]
pub struct GaugeInfoResponse {
    /// Last timestamp when a tuning vote happened
    pub tune_ts: u64,
    /// Distribution of alloc_points to apply in the Generator contract
    pub vamp_points: Vec<(String, Uint128)>,
}

/// The struct describes a response used to return a staker's vAMP lock position.
#[cw_serde]
#[derive(Default)]
pub struct UserInfoResponse {
    /// Last timestamp when the user voted
    pub vote_ts: u64,
    /// The user's decreasing voting power
    pub voting_power: Uint128,
    /// The slope at which the user's voting power decays
    pub slope: Uint128,
    /// Timestamp when the user's lock expires
    pub lock_end: u64,
    /// The vote distribution for all the validators the staker picked
    pub votes: Vec<(String, u16)>,
    /// fixed amount available
    pub fixed_amount: Uint128,
    /// Current voting power at the current
    pub current_power: Uint128,
}

#[cw_serde]
#[derive(Default)]
pub struct UserInfosResponse {
    pub users: Vec<(Addr, UserInfoResponse)>,
}

/// Queries amp tune info.
pub fn get_amp_tune_info(
    querier: &QuerierWrapper,
    amp_gauge_addr: impl Into<String>,
) -> StdResult<GaugeInfoResponse> {
    let gauge: GaugeInfoResponse =
        querier.query_wasm_smart(amp_gauge_addr, &QueryMsg::TuneInfo {})?;
    Ok(gauge)
}

pub fn get_amp_validator_infos(
    querier: &QuerierWrapper,
    amp_gauge_addr: impl Into<String>,
    period: u64,
) -> StdResult<Vec<(String, VotedValidatorInfoResponse)>> {
    let gauge: Vec<(String, VotedValidatorInfoResponse)> = querier.query_wasm_smart(
        amp_gauge_addr,
        &QueryMsg::ValidatorInfos {
            validator_addrs: None,
            period: Some(period),
        },
    )?;
    Ok(gauge)
}
