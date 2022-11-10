use astroport::common::OwnershipProposal;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
use cw_storage_plus::{Item, Map};
use eris::emp_gauges::{ConfigResponse, GaugeInfoResponse};

/// This structure describes the main control config of generator controller contract.
pub type Config = ConfigResponse;

/// This structure describes voting parameters for a specific pool.
#[cw_serde]
#[derive(Default)]
pub struct VotedValidatorInfo {
    /// Dynamic decaying power
    pub voting_power: Uint128,
    /// The slope at which the amount of vxASTRO that voted for this pool/generator will decay
    pub slope: Uint128,
}

/// This structure describes last tuning parameters.
pub type TuneInfo = GaugeInfoResponse;

/// The struct describes last user's votes parameters.
#[cw_serde]
#[derive(Default)]
pub struct EmpInfo {}

/// Stores config at the given key.
pub const CONFIG: Item<Config> = Item::new("config");

/// Stores voting parameters per pool at a specific period by key ( period -> pool_addr ).
pub const VALIDATOR_VOTES: Map<(u64, &str), VotedValidatorInfo> = Map::new("validator_votes");

/// HashSet based on [`Map`]. It contains all pool addresses whose voting power > 0.
pub const VALIDATORS: Map<&str, ()> = Map::new("pools");

/// Hashset based on [`Map`]. It stores null object by key ( pool_addr -> period ).
/// This hashset contains all periods which have saved result in [`POOL_VOTES`] for a specific pool address.
pub const VALIDATOR_PERIODS: Map<(&str, u64), ()> = Map::new("validator_periods");
/// Slope changes for a specific pool address by key ( pool_addr -> period ).
pub const VALIDATOR_SLOPE_CHANGES: Map<(&str, u64), Uint128> = Map::new("validator_slope_changes");

pub const VALIDATOR_FIXED_EMPS: Map<(&str, u64), Uint128> = Map::new("validator_fixed_emps");

// pub const EMP_ID: Item<u64> = Item::new("emp_id");
// pub const EMP_INFOS: Map<u64, EmpInfo> = Map::new("emp_info");

/// Last tuning information.
pub const TUNE_INFO: Item<TuneInfo> = Item::new("tune_info");

/// Contains a proposal to change contract ownership
pub const OWNERSHIP_PROPOSAL: Item<OwnershipProposal> = Item::new("ownership_proposal");
