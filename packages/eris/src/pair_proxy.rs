use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use astroport::asset::{Asset, AssetInfo};

use crate::adapters::router::RouterType;
use cosmwasm_std::Decimal;
use cw20::Cw20ReceiveMsg;

/// Maximum assets in the swap route
pub const MAX_ASSETS: usize = 50;

/// This structure describes the basic settings for creating a contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    /// The list of asset in the swap route
    pub asset_infos: Vec<AssetInfo>,
    /// The router contract address
    pub router: String,
    /// The type of router
    pub router_type: RouterType,
    /// The decimal precision of the offer asset
    pub offer_precision: Option<u8>,
    /// The decimal precision of the ask asset
    pub ask_precision: Option<u8>,
}

/// This structure describes the execute messages of the contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Receives a message of type [`Cw20ReceiveMsg`]
    Receive(Cw20ReceiveMsg),
    /// Swap an offer asset to the other
    Swap {
        /// Offer asset
        offer_asset: Asset,
        /// Belief price of the asset
        belief_price: Option<Decimal>,
        /// Maximum spread from the belief price
        max_spread: Option<Decimal>,
        /// Receiver address
        to: Option<String>,
    },
}

/// ## Description
/// This structure describes a CW20 hook message.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Sell a given amount of asset
    Swap {
        /// Belief price of the asset
        belief_price: Option<Decimal>,
        /// Maximum spread from the belief price        
        max_spread: Option<Decimal>,
        /// Receiver address
        to: Option<String>,
    },
}

/// This structure describes the query messages of the contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Returns information about a pair in an object of type [`super::asset::PairInfo`].
    Pair {},
    /// Returns controls settings that specified in [`Config`] structure.
    Config {},
    /// Returns information about a swap simulation in a [`SimulationResponse`] object.
    Simulation {
        /// Offer asset
        offer_asset: Asset,
        /// Ask asset info when there are more than two assets in the pool
        ask_asset_info: Option<AssetInfo>,
    },
}

/// This structure describes a migration message.
/// We currently take no arguments for migrations.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct MigrateMsg {}
