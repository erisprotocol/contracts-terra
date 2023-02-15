use astroport::asset::{Asset, AssetInfo};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Binary, Decimal, StdResult, Uint128};

/// This structure stores general parameters for the contract.
#[cw_serde]
pub struct InstantiateMsg {
    /// Address that's allowed to update config
    pub owner: String,
    /// Address that's allowed to update bridge assets
    pub operator: String,
    /// The factory contract address
    pub factory_contract: String,
    /// The stablecoin asset info
    pub stablecoin: AssetInfo,
    /// The beneficiary addresses to received fees in stablecoin
    pub target_list: Vec<TargetConfigUnchecked>,
    /// The maximum spread used when swapping fee tokens
    pub max_spread: Option<Decimal>,
}

/// This structure describes the functions that can be executed in this contract.
#[cw_serde]
pub enum ExecuteMsg {
    /// Collects and swaps fee tokens to stablecoin
    Collect {
        /// The assets to swap to stablecoin
        assets: Vec<AssetWithLimit>,
    },
    /// Updates contract config
    UpdateConfig {
        /// The operator address
        operator: Option<String>,
        /// The factory contract address
        factory_contract: Option<String>,
        /// The list of target address to receive fees in stablecoin
        target_list: Option<Vec<TargetConfigUnchecked>>,
        /// The maximum spread used when swapping fee tokens
        max_spread: Option<Decimal>,
    },
    /// Add bridge tokens used to swap specific fee tokens to stablecoin (effectively declaring a swap route)
    UpdateBridges {
        /// List of bridge assets to be added
        add: Option<Vec<(AssetInfo, AssetInfo)>>,
        /// List of asset to be removed
        remove: Option<Vec<AssetInfo>>,
    },
    /// Swap fee tokens via bridge assets
    SwapBridgeAssets {
        assets: Vec<AssetInfo>,
        depth: u64,
    },
    /// Distribute stablecoin to beneficiary
    DistributeFees {},
    /// Creates a request to change the contract's ownership
    ProposeNewOwner {
        /// The newly proposed owner
        owner: String,
        /// The validity period of the proposal to change the owner
        expires_in: u64,
    },
    /// Removes a request to change contract ownership
    DropOwnershipProposal {},
    /// Claims contract ownership
    ClaimOwnership {},
}

/// This structure describes the query functions available in the contract.
#[cw_serde]
pub enum QueryMsg {
    /// Returns information about the maker configs that contains in the [`ConfigResponse`]
    Config {},
    /// Returns the balance for each asset in the specified input parameters
    Balances {
        assets: Vec<AssetInfo>,
    },
    /// Returns list of bridge assets
    Bridges {},
}

/// A custom struct used to return multiple asset balances.
#[cw_serde]
pub struct BalancesResponse {
    /// List of asset and balance in the contract
    pub balances: Vec<Asset>,
}

/// This structure describes a migration message.
/// We currently take no arguments for migrations.
#[cw_serde]
pub struct MigrateMsg {}

/// This struct holds parameters to help with swapping a specific amount of a fee token to ASTRO.
#[cw_serde]
pub struct AssetWithLimit {
    /// Information about the fee token to swap
    pub info: AssetInfo,
    /// The amount of tokens to swap
    pub limit: Option<Uint128>,
}

/// This struct holds parameters to configure receiving contracts and messages.
#[cw_serde]
pub struct TargetConfigUnchecked {
    pub addr: String,
    pub weight: u64,
    pub msg: Option<Binary>,
}

/// This struct holds parameters to configure receiving contracts and messages.
#[cw_serde]
pub struct TargetConfigChecked {
    pub addr: Addr,
    pub weight: u64,
    pub msg: Option<Binary>,
}

impl TargetConfigUnchecked {
    pub fn new(addr: String, weight: u64) -> Self {
        Self {
            addr,
            weight,
            msg: None,
        }
    }

    pub fn new_msg(addr: String, weight: u64, msg: Option<Binary>) -> Self {
        Self {
            addr,
            weight,
            msg,
        }
    }

    pub fn check(&self, api: &dyn Api) -> StdResult<TargetConfigChecked> {
        Ok(TargetConfigChecked {
            addr: api.addr_validate(&self.addr)?,
            weight: self.weight,
            msg: self.msg.clone(),
        })
    }
}

impl TargetConfigChecked {
    pub fn new(addr: Addr, weight: u64) -> Self {
        Self {
            addr,
            weight,
            msg: None,
        }
    }
}
