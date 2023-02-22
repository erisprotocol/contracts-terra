use crate::voting_escrow::QueryMsg::{LockInfo, TotalVamp, TotalVampAt, UserVamp, UserVampAt};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, QuerierWrapper, StdResult, Uint128};
#[allow(unused_imports)]
use cw20::{
    BalanceResponse, Cw20ReceiveMsg, DownloadLogoResponse, Logo, MarketingInfoResponse,
    TokenInfoResponse,
};
use std::fmt;

/// ## Pagination settings
/// The maximum amount of items that can be read at once from
pub const MAX_LIMIT: u32 = 30;

/// The default amount of items to read from
pub const DEFAULT_LIMIT: u32 = 10;

pub const DEFAULT_PERIODS_LIMIT: u64 = 20;

/// This structure stores marketing information for voting escrow.
#[cw_serde]
pub struct UpdateMarketingInfo {
    /// Project URL
    pub project: Option<String>,
    /// Token description
    pub description: Option<String>,
    /// Token marketing information
    pub marketing: Option<String>,
    /// Token logo
    pub logo: Option<Logo>,
}

/// This structure stores general parameters for the voting escrow contract.
#[cw_serde]
pub struct InstantiateMsg {
    /// The voting escrow contract owner
    pub owner: String,
    /// Address that's allowed to black or whitelist contracts
    pub guardian_addr: Option<String>,
    /// ampLP token address
    pub deposit_token_addr: String,
    /// Marketing info for the voting power (vAMP)
    pub marketing: Option<UpdateMarketingInfo>,
    /// The list of whitelisted logo urls prefixes
    pub logo_urls_whitelist: Vec<String>,
}

/// This structure describes the execute functions in the contract.
#[cw_serde]
pub enum ExecuteMsg {
    /// Extend the lockup time for your staked ampLP. For an expired lock, it will always start from the current period.
    ExtendLockTime {
        time: u64,
    },
    /// Receives a message of type [`Cw20ReceiveMsg`] and processes it depending on the received
    /// template.
    Receive(Cw20ReceiveMsg),
    /// Withdraw ampLP from the voting escrow contract
    Withdraw {},
    /// Propose a new owner for the contract
    ProposeNewOwner {
        new_owner: String,
        expires_in: u64,
    },
    /// Remove the ownership transfer proposal
    DropOwnershipProposal {},
    /// Claim contract ownership
    ClaimOwnership {},
    /// Add or remove accounts from the blacklist
    UpdateBlacklist {
        append_addrs: Option<Vec<String>>,
        remove_addrs: Option<Vec<String>>,
    },
    /// Update the marketing info for the voting escrow contract
    UpdateMarketing {
        /// A URL pointing to the project behind this token
        project: Option<String>,
        /// A longer description of the token and its utility. Designed for tooltips or such
        description: Option<String>,
        /// The address (if any) that can update this data structure
        marketing: Option<String>,
    },
    /// Upload a logo for voting escrow
    UploadLogo(Logo),
    /// Update config
    UpdateConfig {
        new_guardian: Option<String>,
        push_update_contracts: Option<Vec<String>>,
    },
    /// Set whitelisted logo urls
    SetLogoUrlsWhitelist {
        whitelist: Vec<String>,
    },
}

#[cw_serde]
pub enum PushExecuteMsg {
    UpdateVote {
        user: String,
        lock_info: LockInfoResponse,
    },
}

/// This structure describes a CW20 hook message.
#[cw_serde]
pub enum Cw20HookMsg {
    /// Create a vAMP position and lock ampLP for `time` amount of time
    CreateLock {
        time: u64,
    },
    /// Deposit ampLP in another user's vAMP position
    DepositFor {
        user: String,
    },
    /// Add more ampLP to your vAMP position
    ExtendLockAmount {
        /// Specify that the contract should extend the lock time to the min required periods
        extend_to_min_periods: Option<bool>,
    },
}

/// This enum describes voters status.
#[cw_serde]
pub enum BlacklistedVotersResponse {
    /// Voters are blacklisted
    VotersBlacklisted {},
    /// Returns a voter that is not blacklisted.
    VotersNotBlacklisted {
        voter: String,
    },
}

impl fmt::Display for BlacklistedVotersResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BlacklistedVotersResponse::VotersBlacklisted {} => write!(f, "Voters are blacklisted!"),
            BlacklistedVotersResponse::VotersNotBlacklisted {
                voter,
            } => {
                write!(f, "Voter is not blacklisted: {}", voter)
            },
        }
    }
}

/// This structure describes the query messages available in the contract.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Checks if specified addresses are blacklisted
    #[returns(BlacklistedVotersResponse)]
    CheckVotersAreBlacklisted {
        voters: Vec<String>,
    },
    /// Return the blacklisted voters
    #[returns(Vec<Addr>)]
    BlacklistedVoters {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Return the user's vAMP balance
    #[returns(BalanceResponse)]
    Balance {
        address: String,
    },
    /// Fetch the vAMP token information
    #[returns(TokenInfoResponse)]
    TokenInfo {},
    /// Fetch vAMP's marketing information
    #[returns(MarketingInfoResponse)]
    MarketingInfo {},
    /// Download the vAMP logo
    #[returns(DownloadLogoResponse)]
    DownloadLogo {},
    /// Return the current total amount of vAMP
    #[returns(VotingPowerResponse)]
    TotalVamp {},
    /// Return the total amount of vAMP at some point in the past
    #[returns(VotingPowerResponse)]
    TotalVampAt {
        time: u64,
    },
    /// Return the total voting power at a specific period
    #[returns(VotingPowerResponse)]
    TotalVampAtPeriod {
        period: u64,
    },
    /// Return the user's current voting power (vAMP balance)
    #[returns(VotingPowerResponse)]
    UserVamp {
        user: String,
    },
    /// Return the user's vAMP balance at some point in the past
    #[returns(VotingPowerResponse)]
    UserVampAt {
        user: String,
        time: u64,
    },
    /// Return the user's voting power at a specific period
    #[returns(VotingPowerResponse)]
    UserVampAtPeriod {
        user: String,
        period: u64,
    },
    /// Return information about a user's lock position
    #[returns(LockInfoResponse)]
    LockInfo {
        user: String,
    },
    /// Return user's locked ampLP balance at the given block height
    #[returns(Uint128)]
    UserDepositAtHeight {
        user: String,
        height: u64,
    },
    /// Return the vAMP contract configuration
    #[returns(ConfigResponse)]
    Config {},
}

/// This structure is used to return a user's amount of vAMP.
#[cw_serde]
pub struct VotingPowerResponse {
    /// The vAMP balance
    pub vamp: Uint128,
}

/// This structure is used to return the lock information for a vAMP position.
#[cw_serde]
pub struct LockInfoResponse {
    /// The amount of ampLP locked in the position
    pub amount: Uint128,
    /// This is the initial boost for the lock position
    pub coefficient: Decimal,
    /// Start time for the vAMP position decay
    pub start: u64,
    /// End time for the vAMP position decay
    pub end: u64,
    /// Slope at which a staker's vAMP balance decreases over time
    pub slope: Uint128,

    /// fixed sockel
    pub fixed_amount: Uint128,
    /// includes only decreasing voting_power, it is the current voting power of the period currently queried.
    pub voting_power: Uint128,
}

/// This structure stores the parameters returned when querying for a contract's configuration.
#[cw_serde]
pub struct ConfigResponse {
    /// Address that's allowed to change contract parameters
    pub owner: String,
    /// Address that can only blacklist vAMP stakers and remove their governance power
    pub guardian_addr: Option<Addr>,
    /// The ampLP token contract address
    pub deposit_token_addr: String,
    /// The list of whitelisted logo urls prefixes
    pub logo_urls_whitelist: Vec<String>,
    /// The list of contracts to receive push updates
    pub push_update_contracts: Vec<String>,
}

/// This structure describes a Migration message.
#[cw_serde]
pub struct MigrateMsg {}

/// Queries current user's voting power from the voting escrow contract.
///
/// * **user** staker for which we calculate the latest vAMP voting power.
pub fn get_voting_power(
    querier: &QuerierWrapper,
    escrow_addr: impl Into<String>,
    user: impl Into<String>,
) -> StdResult<Uint128> {
    let vp: VotingPowerResponse = querier.query_wasm_smart(
        escrow_addr,
        &UserVamp {
            user: user.into(),
        },
    )?;
    Ok(vp.vamp)
}

/// Queries current user's voting power from the voting escrow contract by timestamp.
///
/// * **user** staker for which we calculate the voting power at a specific time.
///
/// * **timestamp** timestamp at which we calculate the staker's voting power.
pub fn get_voting_power_at(
    querier: &QuerierWrapper,
    escrow_addr: impl Into<String>,
    user: impl Into<String>,
    timestamp: u64,
) -> StdResult<Uint128> {
    let vp: VotingPowerResponse = querier.query_wasm_smart(
        escrow_addr,
        &UserVampAt {
            user: user.into(),
            time: timestamp,
        },
    )?;

    Ok(vp.vamp)
}

/// Queries current total voting power from the voting escrow contract.
pub fn get_total_voting_power(
    querier: &QuerierWrapper,
    escrow_addr: impl Into<String>,
) -> StdResult<Uint128> {
    let vp: VotingPowerResponse = querier.query_wasm_smart(escrow_addr, &TotalVamp {})?;

    Ok(vp.vamp)
}

/// Queries total voting power from the voting escrow contract by timestamp.
///
/// * **timestamp** time at which we fetch the total voting power.
pub fn get_total_voting_power_at(
    querier: &QuerierWrapper,
    escrow_addr: impl Into<String>,
    timestamp: u64,
) -> StdResult<Uint128> {
    let vp: VotingPowerResponse = querier.query_wasm_smart(
        escrow_addr,
        &TotalVampAt {
            time: timestamp,
        },
    )?;

    Ok(vp.vamp)
}

/// Queries total voting power from the voting escrow contract by period.
///
/// * **timestamp** time at which we fetch the total voting power.
pub fn get_total_voting_power_at_by_period(
    querier: &QuerierWrapper,
    escrow_addr: impl Into<String>,
    period: u64,
) -> StdResult<Uint128> {
    let vp: VotingPowerResponse = querier.query_wasm_smart(
        escrow_addr,
        &QueryMsg::TotalVampAtPeriod {
            period,
        },
    )?;

    Ok(vp.vamp)
}

/// Queries user's lockup information from the voting escrow contract.
///
/// * **user** staker for which we return lock position information.
pub fn get_lock_info(
    querier: &QuerierWrapper,
    escrow_addr: impl Into<String>,
    user: impl Into<String>,
) -> StdResult<LockInfoResponse> {
    let lock_info: LockInfoResponse = querier.query_wasm_smart(
        escrow_addr,
        &LockInfo {
            user: user.into(),
        },
    )?;
    Ok(lock_info)
}
