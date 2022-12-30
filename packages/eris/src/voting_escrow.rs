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

/// This structure stores marketing information for vxASTRO.
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

/// This structure stores general parameters for the vxASTRO contract.
#[cw_serde]
pub struct InstantiateMsg {
    /// The vxASTRO contract owner
    pub owner: String,
    /// Address that's allowed to black or whitelist contracts
    pub guardian_addr: Option<String>,
    /// xASTRO token address
    pub deposit_token_addr: String,
    /// Marketing info for vxASTRO
    pub marketing: Option<UpdateMarketingInfo>,
    /// The list of whitelisted logo urls prefixes
    pub logo_urls_whitelist: Vec<String>,
}

/// This structure describes the execute functions in the contract.
#[cw_serde]
pub enum ExecuteMsg {
    /// Extend the lockup time for your staked xASTRO
    ExtendLockTime {
        time: u64,
    },
    /// Receives a message of type [`Cw20ReceiveMsg`] and processes it depending on the received
    /// template.
    Receive(Cw20ReceiveMsg),
    /// Withdraw xASTRO from the vxASTRO contract
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
    /// Update the marketing info for the vxASTRO contract
    UpdateMarketing {
        /// A URL pointing to the project behind this token
        project: Option<String>,
        /// A longer description of the token and its utility. Designed for tooltips or such
        description: Option<String>,
        /// The address (if any) that can update this data structure
        marketing: Option<String>,
    },
    /// Upload a logo for vxASTRO
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
    /// Create a vxASTRO position and lock xASTRO for `time` amount of time
    CreateLock {
        time: u64,
    },
    /// Deposit xASTRO in another user's vxASTRO position
    DepositFor {
        user: String,
    },
    /// Add more xASTRO to your vxASTRO position
    ExtendLockAmount {},
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
    /// Return the user's vxASTRO balance
    #[returns(BalanceResponse)]
    Balance {
        address: String,
    },
    /// Fetch the vxASTRO token information
    #[returns(TokenInfoResponse)]
    TokenInfo {},
    /// Fetch vxASTRO's marketing information
    #[returns(MarketingInfoResponse)]
    MarketingInfo {},
    /// Download the vxASTRO logo
    #[returns(DownloadLogoResponse)]
    DownloadLogo {},
    /// Return the current total amount of vxASTRO
    #[returns(VotingPowerResponse)]
    TotalVamp {},
    /// Return the total amount of vxASTRO at some point in the past
    #[returns(VotingPowerResponse)]
    TotalVampAt {
        time: u64,
    },
    /// Return the total voting power at a specific period
    #[returns(VotingPowerResponse)]
    TotalVampAtPeriod {
        period: u64,
    },
    /// Return the user's current voting power (vxASTRO balance)
    #[returns(VotingPowerResponse)]
    UserVamp {
        user: String,
    },
    /// Return the user's vxASTRO balance at some point in the past
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
    /// Return user's locked xASTRO balance at the given block height
    #[returns(Uint128)]
    UserDepositAtHeight {
        user: String,
        height: u64,
    },
    /// Return the  vxASTRO contract configuration
    #[returns(ConfigResponse)]
    Config {},
}

/// This structure is used to return a user's amount of vxASTRO.
#[cw_serde]
pub struct VotingPowerResponse {
    /// The vxASTRO balance
    pub vamp: Uint128,
}

/// This structure is used to return the lock information for a vxASTRO position.
#[cw_serde]
pub struct LockInfoResponse {
    /// The amount of xASTRO locked in the position
    pub amount: Uint128,
    /// This is the initial boost for the lock position
    pub coefficient: Decimal,
    /// Start time for the vxASTRO position decay
    pub start: u64,
    /// End time for the vxASTRO position decay
    pub end: u64,
    /// Slope at which a staker's vxASTRO balance decreases over time
    pub slope: Uint128,

    /// fixed sockel
    pub fixed_amount: Uint128,
    /// includes only decreasing voting_power
    pub voting_power: Uint128,
}

/// This structure stores the parameters returned when querying for a contract's configuration.
#[cw_serde]
pub struct ConfigResponse {
    /// Address that's allowed to change contract parameters
    pub owner: String,
    /// Address that can only blacklist vxASTRO stakers and remove their governance power
    pub guardian_addr: Option<Addr>,
    /// The xASTRO token contract address
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
/// * **user** staker for which we calculate the latest vxASTRO voting power.
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