use std::{
    convert::TryFrom,
    ops::{Div, Mul},
    vec,
};

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
    /// Specifies wether voting should be weighted based on VP
    pub use_weighted_vote: bool,
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

        /// Updates if weighted voting is used
        use_weighted_vote: Option<bool>,
    },
    // Admin action to remove a user
    RemoveUser {
        user: String,
    },
    // Admin action to remove a user
    RemoveProp {
        proposal_id: u64,
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

    /// UserInfo returns information about a voter and the validators they voted for
    #[returns(UserVotesResponse)]
    UserVotes {
        user: String,
        start_after: Option<u64>,
        limit: Option<u32>,
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

    #[serde(default)]
    pub use_weighted_vote: bool,
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

    #[serde(default)]
    pub total_vp: Uint128,

    pub current_vote: Option<VoteOption>,
}

impl PropInfo {
    pub fn voted_vp(&self) -> Uint128 {
        self.yes_vp + self.no_vp + self.abstain_vp + self.nwv_vp
    }

    pub fn reached_quorum(&self, total_vp: Uint128, quorum: u16) -> StdResult<bool> {
        let current = self.voted_vp();
        let voted = Decimal::from_ratio(current, total_vp);
        let quorum = BasicPoints::try_from(quorum)?.decimal();

        if voted < quorum {
            Ok(false)
        } else {
            Ok(true)
        }
    }

    pub fn get_weighted_votes(&self) -> Vec<(Decimal, VoteOption)> {
        let voted = self.voted_vp();

        if voted.is_zero() {
            return vec![];
        }

        let rounding_factor = Uint128::new(1000);

        let mut votes = vec![
            (
                Decimal::from_ratio(self.yes_vp.mul(rounding_factor), voted)
                    .floor()
                    .div(rounding_factor),
                VoteOption::Yes,
            ),
            (
                Decimal::from_ratio(self.no_vp.mul(rounding_factor), voted)
                    .floor()
                    .div(rounding_factor),
                VoteOption::No,
            ),
            (
                Decimal::from_ratio(self.abstain_vp.mul(rounding_factor), voted)
                    .floor()
                    .div(rounding_factor),
                VoteOption::Abstain,
            ),
            (
                Decimal::from_ratio(self.nwv_vp.mul(rounding_factor), voted)
                    .floor()
                    .div(rounding_factor),
                VoteOption::NoWithVeto,
            ),
        ]
        .into_iter()
        .filter(|a| !a.0.is_zero())
        .collect::<Vec<(Decimal, VoteOption)>>();

        let sum: Decimal = votes.iter().map(|a| a.0).sum();

        match sum.cmp(&Decimal::one()) {
            std::cmp::Ordering::Less => {
                let too_less = Decimal::one() - sum;
                votes[0].0 += too_less;
            },
            std::cmp::Ordering::Greater => {
                let too_much = sum - Decimal::one();
                votes[0].0 -= too_much;
            },
            _ => {},
        }

        votes
    }

    pub fn get_wanted_vote(&self, total_vp: Uint128, quorum: u16) -> StdResult<Option<VoteOption>> {
        let current = self.voted_vp();
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

#[cw_serde]
pub struct UserVotesResponse {
    pub props: Vec<UserPropResponseItem>,
}

#[cw_serde]
pub struct UserPropResponseItem {
    pub id: u64,
    pub current_vote: VoteOption,
    pub vp: Uint128,
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
    #[serde(default = "default_addr")]
    pub user: Addr,
    pub current_vote: VoteOption,
    pub vp: Uint128,
}

fn default_addr() -> Addr {
    Addr::unchecked("")
}

#[test]
fn test_weighted_votes_empty() {
    let prop = PropInfo {
        abstain_vp: Uint128::zero(),
        period: 100,
        end_time_s: 100,
        yes_vp: Uint128::zero(),
        no_vp: Uint128::zero(),
        nwv_vp: Uint128::zero(),
        total_vp: Uint128::zero(),
        current_vote: None,
    };

    let votes = prop.get_weighted_votes();
    assert_eq!(votes, vec![]);
}

#[test]
fn test_weighted_votes_some() {
    let prop = PropInfo {
        abstain_vp: Uint128::zero(),
        period: 100,
        end_time_s: 100,
        yes_vp: Uint128::new(10),
        no_vp: Uint128::new(20),
        nwv_vp: Uint128::zero(),
        total_vp: Uint128::zero(),
        current_vote: None,
    };

    let votes = prop.get_weighted_votes();
    assert_eq!(
        votes.iter().map(|a| a.0.to_string()).collect::<Vec<String>>(),
        vec!["0.334".to_string(), "0.666".to_string()]
    );
}
#[test]
fn test_weighted_votes_single() {
    let prop = PropInfo {
        abstain_vp: Uint128::zero(),
        period: 100,
        end_time_s: 100,
        yes_vp: Uint128::new(9907854),
        no_vp: Uint128::zero(),
        nwv_vp: Uint128::zero(),
        total_vp: Uint128::zero(),
        current_vote: None,
    };

    let votes = prop.get_weighted_votes();
    assert_eq!(
        votes.iter().map(|a| a.0.to_string()).collect::<Vec<String>>(),
        vec!["1".to_string()]
    );
}
