use astroport::common::OwnershipProposal;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use eris::amp_gauges::{
    ConfigResponse, GaugeInfoResponse, UserInfoResponse, VotedValidatorInfoResponse,
};
use eris::helpers::bps::BasicPoints;

/// This structure describes the main control config of generator controller contract.
pub type Config = ConfigResponse;
/// This structure describes voting parameters for a specific validator.
pub type VotedValidatorInfo = VotedValidatorInfoResponse;
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
    pub votes: Vec<(Addr, BasicPoints)>,
    pub fixed_amount: Uint128,
}

impl UserInfo {
    /// The function converts [`UserInfo`] object into [`UserInfoResponse`].
    pub(crate) fn into_response(self) -> UserInfoResponse {
        let votes = self
            .votes
            .iter()
            .map(|(validator_addr, bps)| (validator_addr.clone(), u16::from(*bps)))
            .collect();

        UserInfoResponse {
            vote_ts: self.vote_ts,
            voting_power: self.voting_power,
            slope: self.slope,
            lock_end: self.lock_end,
            votes,
        }
    }
}

/// Stores config at the given key.
pub const CONFIG: Item<Config> = Item::new("config");

/// Stores voting parameters per pool at a specific period by key ( period -> validator_addr ).
pub const VALIDATOR_VOTES: Map<(u64, &Addr), VotedValidatorInfo> = Map::new("validator_votes");

/// HashSet based on [`Map`]. It contains all validator addresses whose voting power > 0.
pub const VALIDATORS: Map<&Addr, ()> = Map::new("validators");

/// Hashset based on [`Map`]. It stores null object by key ( validator_addr -> period ).
/// This hashset contains all periods which have saved result in [`VALIDATOR_VOTES`] for a specific validator address.
pub const VALIDATOR_PERIODS: Map<(&Addr, u64), ()> = Map::new("validator_periods");

/// Slope changes for a specific validator address by key ( validator_addr -> period ).
pub const VALIDATOR_SLOPE_CHANGES: Map<(&Addr, u64), Uint128> = Map::new("validator_slope_changes");

pub const VALIDATOR_FIXED_VAMP: Map<(&Addr, u64), Uint128> = Map::new("validator_fixed_vamp");

/// User's voting information.
pub const USER_INFO: Map<&Addr, UserInfo> = Map::new("user_info");

/// Last tuning information.
pub const TUNE_INFO: Item<TuneInfo> = Item::new("tune_info");

/// Contains a proposal to change contract ownership
pub const OWNERSHIP_PROPOSAL: Item<OwnershipProposal> = Item::new("ownership_proposal");

/// Slope changes for a specific validator address by key ( validator_addr -> period ).
pub const VALIDATOR_EMP_SLOPE_CHANGES: Map<(&Addr, u64), Uint128> =
    Map::new("validator_emp_slope_changes");
