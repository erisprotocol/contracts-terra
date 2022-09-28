use astroport::asset::{AssetInfo, PairInfo};
use cw_storage_plus::Item;
use eris::adapters::router::{Router, RouterType};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// ## Description
/// This structure describes the main control config of pair.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub pair_info: PairInfo,
    pub asset_infos: Vec<AssetInfo>,
    pub router: Router,
    pub router_type: RouterType,
    pub offer_precision: u8,
    pub ask_precision: u8,
}

/// ## Description
/// Stores config at the given key
pub const CONFIG: Item<Config> = Item::new("config");
