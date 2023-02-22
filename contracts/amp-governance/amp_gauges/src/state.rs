use astroport::common::OwnershipProposal;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, StdResult, Uint128};
use cw_storage_plus::{Item, Map};
use eris::amp_gauges::{ConfigResponse, GaugeInfoResponse, UserInfoResponse};
use eris::governance_helper::{calc_voting_power, get_period};
use eris::helpers::bps::BasicPoints;

/// This structure describes the main control config of generator controller contract.
pub type Config = ConfigResponse;
/// This structure describes voting parameters for a specific validator.
#[cw_serde]
#[derive(Default)]
pub struct VotedValidatorInfo {
    /// voting_power for this validator
    pub voting_power: Uint128,
    /// The slope at which the amount of vAMP that voted for this validator will decay
    pub slope: Uint128,
}

/// This structure describes last tuning parameters.
pub type TuneInfo = GaugeInfoResponse;

/// The struct describes last user's votes parameters.
#[cw_serde]
#[derive(Default)]
pub struct UserInfo {
    pub vote_ts: u64,
    pub voting_power: Uint128,
    pub slope: Uint128,
    pub lock_end: u64,
    pub votes: Vec<(String, BasicPoints)>,
    pub fixed_amount: Uint128,
}

impl UserInfo {
    /// The function converts [`UserInfo`] object into [`UserInfoResponse`].
    pub(crate) fn into_response(self, period: u64) -> StdResult<UserInfoResponse> {
        let votes = self
            .votes
            .into_iter()
            .map(|(validator_addr, bps)| (validator_addr, u16::from(bps)))
            .collect();

        let user_last_vote_period = get_period(self.vote_ts).unwrap_or(period);
        let vp_at_period =
            calc_voting_power(self.slope, self.voting_power, user_last_vote_period, period);

        Ok(UserInfoResponse {
            vote_ts: self.vote_ts,
            voting_power: self.voting_power,
            slope: self.slope,
            lock_end: self.lock_end,
            votes,
            fixed_amount: self.fixed_amount,
            current_power: self.fixed_amount.checked_add(vp_at_period)?,
        })
    }
}

/// Stores config at the given key.
pub const CONFIG: Item<Config> = Item::new("config");

/// Stores voting parameters per pool at a specific period by key ( period -> validator_addr ).
pub const VALIDATOR_VOTES: Map<(u64, &str), VotedValidatorInfo> = Map::new("validator_votes");

/// HashSet based on [`Map`]. It contains all validator addresses whose voting power > 0.
pub const VALIDATORS: Map<&str, ()> = Map::new("validators");

/// Hashset based on [`Map`]. It stores null object by key ( validator_addr -> period ).
/// This hashset contains all periods which have saved result in [`VALIDATOR_VOTES`] for a specific validator address.
pub const VALIDATOR_PERIODS: Map<(&str, u64), ()> = Map::new("validator_periods");

/// Slope changes for a specific validator address by key ( validator_addr -> period ).
pub const VALIDATOR_SLOPE_CHANGES: Map<(&str, u64), Uint128> = Map::new("validator_slope_changes");

pub const VALIDATOR_FIXED_VAMP: Map<(&str, u64), Uint128> = Map::new("validator_fixed_vamp");

/// User's voting information.
pub const USER_INFO: Map<&Addr, UserInfo> = Map::new("user_info");

/// Last tuning information.
pub const TUNE_INFO: Item<TuneInfo> = Item::new("tune_info");

/// Contains a proposal to change contract ownership
pub const OWNERSHIP_PROPOSAL: Item<OwnershipProposal> = Item::new("ownership_proposal");
