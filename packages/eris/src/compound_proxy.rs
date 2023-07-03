use cosmwasm_schema::{cw_serde, QueryResponses};

use astroport::asset::{Asset, AssetInfo};

use cosmwasm_std::{to_binary, Addr, CosmosMsg, Decimal, StdResult, Uint128, WasmMsg};

use crate::adapters::router::RouterType;

/// This structure describes the basic settings for creating a contract.
#[cw_serde]
pub struct InstantiateMsg {
    // supported LPs
    pub lps: Vec<LpInit>,
    // supported Routes
    pub routes: Vec<RouteInit>,
    // allowed factory
    pub factory: Option<String>,
    // owner
    pub owner: String,
}

#[cw_serde]
pub struct LpInit {
    /// The pair info
    pub pair_contract: String,
    /// The swap commission
    pub commission_bps: u64,
    /// The slippage tolerance when providing liquidity
    pub slippage_tolerance: Decimal,
    /// Token used for providing liquidity
    pub wanted_token: AssetInfo,

    // default: astroport
    pub lp_type: Option<LpType>,
}

/// This structure stores the main parameters for an Astroport pair
#[cw_serde]
pub struct PairInfo {
    /// Asset information for the assets in the pool
    pub asset_infos: Vec<AssetInfo>,
    /// Pair contract address
    pub contract_addr: Addr,
    /// Pair LP token address
    pub liquidity_token: Addr,
    /// The pool type (xyk, stableswap etc) available in [`PairType`]
    pub pair_type: PairType,
}

#[cw_serde]
pub struct PairInfoWw {
    pub asset_infos: [AssetInfo; 2],
    pub contract_addr: String,
    pub liquidity_token: AssetInfo,
    pub asset_decimals: [u8; 2],
    pub pair_type: PairTypeWw,
}

#[cw_serde]
pub enum PairTypeWw {
    StableSwap {
        /// The amount of amplification to perform on the constant product part of the swap formula.
        amp: u64,
    },
    ConstantProduct,
}

impl PairInfo {
    /// Returns the balance for each asset in the pool.
    /// ## Params
    /// * **self** is the type of the caller object
    ///
    /// * **querier** is an object of type [`QuerierWrapper`]
    ///
    /// * **contract_addr** is pair's pool address.
    pub fn query_pools(
        &self,
        querier: &cosmwasm_std::QuerierWrapper,
        contract_addr: impl Into<String>,
    ) -> StdResult<Vec<Asset>> {
        let contract_addr = contract_addr.into();
        self.asset_infos
            .iter()
            .map(|asset_info| {
                Ok(Asset {
                    info: asset_info.clone(),
                    amount: asset_info.query_pool(querier, &contract_addr)?,
                })
            })
            .collect()
    }
}

#[cw_serde]
pub enum PairType {
    /// XYK pair type
    Xyk {},
    /// Stable pair type
    Stable {},
    /// Custom pair type
    Custom(String),

    /// Stable pair type
    StableWhiteWhale {},
    /// XYK pair type
    XykWhiteWhale {},
}

#[cw_serde]
pub enum LpType {
    Astroport,
    WhiteWhale,
}

#[cw_serde]
pub enum RouteInit {
    Path {
        router: String,
        router_type: RouterType,
        route: Vec<AssetInfo>,
    },
    PairProxy {
        /// when specified, a pair can be defined as a single direction
        single_direction_from: Option<AssetInfo>,
        pair_contract: String,
    },
}

#[cw_serde]
pub struct RouteDelete {
    pub from: AssetInfo,
    pub to: AssetInfo,
    // specifies wether also to->from should be removed. default: true
    pub both: Option<bool>,
}

/// This structure describes the execute messages of the contract.
#[cw_serde]
pub enum ExecuteMsg {
    // /// Implements the Cw20 receiver interface
    // Receive(Cw20ReceiveMsg),
    /// Compound rewards to LP token
    Compound {
        /// LP into which the assets should be compounded into
        lp_token: String,
        /// List of reward asset send to compound
        rewards: Vec<Asset>,
        /// Receiver address for LP token
        receiver: Option<String>,
        /// Skip optimal swap
        no_swap: Option<bool>,
        /// slippage tolerance when providing LP
        slippage_tolerance: Option<Decimal>,
    },
    /// Swaps a number of assets to a single result
    MultiSwap {
        /// LP into which the assets should be compounded into
        into: AssetInfo,
        /// List of reward asset send to compound
        assets: Vec<Asset>,
        /// Receiver address for LP token
        receiver: Option<String>,
    },
    /// Creates a request to change the contract's ownership
    ProposeNewOwner {
        /// The newly proposed owner
        owner: String,
        /// The validity period of the proposal to change the owner
        expires_in: u64,
    },

    UpdateConfig {
        factory: Option<String>,
        remove_factory: Option<bool>,

        upsert_lps: Option<Vec<LpInit>>,
        delete_lps: Option<Vec<String>>,
        insert_routes: Option<Vec<RouteInit>>,
        delete_routes: Option<Vec<RouteDelete>>,

        // default slippage, 50 = 50%, max = 50
        default_max_spread: Option<u64>,
    },

    /// Removes a request to change contract ownership
    DropOwnershipProposal {},
    /// Claims contract ownership
    ClaimOwnership {},

    /// The callback of type [`CallbackMsg`]
    Callback(CallbackMsg),
}

#[cw_serde]
pub enum ReceiveMsg {
    /// splits an asset into it's parts and then converts them to the wanted result
    Split {
        /// LP into which the assets should be compounded into
        into: AssetInfo,
        /// Receiver address for LP token
        receiver: Option<String>,
        /// slippage tolerance when providing LP
        slippage_tolerance: Option<Decimal>,
    },
}

/// This structure describes the callback messages of the contract.
#[cw_serde]
pub enum CallbackMsg {
    /// Performs optimal swap
    OptimalSwap {
        lp_token: String,
    },
    /// Provides liquidity to the pair contract
    ProvideLiquidity {
        prev_balances: Vec<Asset>,
        receiver: String,
        slippage_tolerance: Option<Decimal>,
        lp_token: String,
    },
    SendSwapResult {
        token: AssetInfo,
        receiver: String,
    },
}

// Modified from
// https://github.com/CosmWasm/cw-plus/blob/v0.8.0/packages/cw20/src/receiver.rs#L23
impl CallbackMsg {
    pub fn into_cosmos_msg(&self, contract_addr: &Addr) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: String::from(contract_addr),
            msg: to_binary(&ExecuteMsg::Callback(self.clone()))?,
            funds: vec![],
        }))
    }
}

/// This structure describes the query messages of the contract.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns controls settings that specified in custom [`ConfigResponse`] structure.
    #[returns(ConfigResponse)]
    Config {},
    /// Return LP token amount received after compound
    #[returns(CompoundSimulationResponse)]
    CompoundSimulation {
        rewards: Vec<Asset>,
        lp_token: String,
    },
    #[returns(LpConfig)]
    GetLp {
        lp_addr: String,
    },
    // returns the state and assets of a pair by using the LP token addr
    #[returns(LpStateResponse)]
    GetLpState {
        lp_addr: String,
    },
    // return all allowed lps
    #[returns(Vec<LpConfig>)]
    GetLps {
        // start after the provided liquidity_token
        start_after: Option<String>,
        limit: Option<u32>,
    },
    // return all known pairs
    #[returns(Vec<RouteResponseItem>)]
    GetRoutes {
        start_after: Option<(AssetInfo, AssetInfo)>,
        limit: Option<u32>,
    },
    // return a single route
    #[returns(RouteResponseItem)]
    GetRoute {
        from: AssetInfo,
        to: AssetInfo,
    },

    #[returns(SupportsSwapResponse)]
    SupportsSwap {
        from: AssetInfo,
        to: AssetInfo,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    pub factory: Option<Addr>,
    pub owner: Addr,
}

/// This structure holds the parameters that are returned from a compound simulation response
#[cw_serde]
pub struct CompoundSimulationResponse {
    /// The amount of LP returned from compound
    pub lp_amount: Uint128,
    /// The amount of asset A to be swapped
    pub swap_asset_a_amount: Uint128,
    /// The amount of asset B to be swapped
    pub swap_asset_b_amount: Uint128,
    /// The amount of asset A returned from swap
    pub return_a_amount: Uint128,
    /// The amount of asset B returned from swap
    pub return_b_amount: Uint128,
}

#[cw_serde]
pub struct LpConfig {
    /// The pair info
    pub pair_info: PairInfo,
    /// The swap commission for the LP pair
    pub commission_bps: u64,
    /// The slippage tolerance when providing liquidity
    pub slippage_tolerance: Decimal,
    /// Token used for providing liquidity
    pub wanted_token: AssetInfo,
}

#[cw_serde]
pub struct LpStateResponse {
    /// Pair contract address
    pub contract_addr: Addr,
    /// Pair LP token address
    pub liquidity_token: Addr,
    /// The assets in the pool together with asset amounts
    pub assets: Vec<Asset>,
    /// The total amount of LP tokens currently issued
    pub total_share: Uint128,
}

#[cw_serde]
pub struct SupportsSwapResponse {
    pub suppored: bool,
}

/// This structure describes a migration message.
/// We currently take no arguments for migrations.
#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct RouteResponseItem {
    pub key: (String, String),
    pub route_type: RouteTypeResponseItem,
}

#[cw_serde]
pub enum RouteTypeResponseItem {
    Path {
        router: String,
        router_type: RouterType,
        route: Vec<String>,
    },
    PairProxy {
        pair_contract: String,
        asset_infos: Vec<String>,
    },
}
