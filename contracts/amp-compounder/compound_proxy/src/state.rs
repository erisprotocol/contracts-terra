use astroport::asset::PairInfo;
use cosmwasm_std::Decimal;
use cw_storage_plus::{Item, Map};
use eris::adapters::pair::Pair;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// This structure describes the main control config of pair.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// The pair info
    pub pair_info: PairInfo,
    /// The swap commission
    pub commission_bps: u64,
    /// The slippage tolerance when providing liquidity
    pub slippage_tolerance: Decimal,
}

/// Stores config at the given key
pub const CONFIG: Item<Config> = Item::new("config");

/// Stores pair proxy for the given reward
pub const PAIR_PROXY: Map<String, Pair> = Map::new("pair_proxy");
