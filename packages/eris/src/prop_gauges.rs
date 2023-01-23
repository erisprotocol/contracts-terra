use std::convert::TryFrom;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, StdError, StdResult, Uint128, VoteOption};

use crate::{helpers::bps::BasicPoints, voting_escrow::LockInfoResponse};

/// This structure describes the basic settings for creating a contract.
#[cw_serde]
pub struct InstantiateMsg {
    /// Contract owner
    pub owner: String,
    /// The vAMP token contract address
    pub escrow_addr: String,
    /// Hub contract address
    pub hub_addr: String,

    /// Min voting power required
    pub quorum_bps: u16,
}

/// This structure describes the execute messages available in the contract.
#[cw_serde]
pub enum ExecuteMsg {
    InitProp {
        proposal_id: u64,
        end_time_s: u64,
    },

    /// Vote allows a vAMP holder to cast votes on which validators should get the delegations
    Vote {
        proposal_id: u64,
        vote: VoteOption,
    },

    /// Updates the vote for a specified user. Only can be called from the escrow_addr
    UpdateVote {
        user: String,
        lock_info: LockInfoResponse,
    },

    UpdateConfig {
        /// ChangeValidatorsLimit changes the max amount of validators that can be voted at once to receive delegations
        quorum_bps: Option<u16>,
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
    /// Config returns the contract configuration
    #[returns(ConfigResponse)]
    Config {},

    /// Returns all props that can be voted on (ascending order)
    #[returns(PropsResponse)]
    ActiveProps {
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    /// Returns all props that have finished (descending order)
    #[returns(PropsResponse)]
    FinishedProps {
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    /// UserInfo returns information about a voter and the validators they voted for
    #[returns(PropDetailResponse)]
    PropDetail {
        user: Option<String>,
        proposal_id: u64,
    },

    #[returns(PropVotersResponse)]
    PropVoters {
        proposal_id: u64,
        start_after: Option<(u128, String)>,
        limit: Option<u32>,
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

    /// Required min quorum (voted voting power / total voting power must be > quorum to allow the contract to vote)
    pub quorum_bps: u16,
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

#[cw_serde]
pub struct PropInfo {
    pub period: u64,
    pub end_time_s: u64,

    pub yes_vp: Uint128,
    pub no_vp: Uint128,
    pub abstain_vp: Uint128,
    pub nwv_vp: Uint128,

    pub current_vote: Option<VoteOption>,
}

impl PropInfo {
    pub fn get_wanted_vote(&self, total_vp: Uint128, quorum: u16) -> StdResult<Option<VoteOption>> {
        let current = self.yes_vp + self.no_vp + self.abstain_vp + self.nwv_vp;

        let voted = Decimal::from_ratio(current, total_vp);
        let quorum = BasicPoints::try_from(quorum)?.decimal();

        let result = if voted < quorum {
            None
        } else if self.yes_vp >= self.no_vp
            && self.yes_vp >= self.abstain_vp
            && self.yes_vp >= self.nwv_vp
        {
            Some(VoteOption::Yes)
        } else if self.no_vp >= self.abstain_vp && self.no_vp >= self.nwv_vp {
            Some(VoteOption::No)
        } else if self.nwv_vp >= self.abstain_vp {
            Some(VoteOption::NoWithVeto)
        } else {
            Some(VoteOption::Abstain)
        };

        Ok(result)
    }
}

#[cw_serde]
pub struct PropsResponse {
    pub props: Vec<(u64, PropInfo)>,
}

/// The struct describes a response used to return a staker's vAMP lock position.
#[cw_serde]
pub struct PropDetailResponse {
    pub prop: PropInfo,
    pub user: Option<PropUserInfo>,
}

#[cw_serde]
pub struct PropVotersResponse {
    pub voters: Vec<(u128, Addr, VoteOption)>,
}

#[cw_serde]
pub struct PropUserInfo {
    pub current_vote: VoteOption,
    pub vp: Uint128,
}
