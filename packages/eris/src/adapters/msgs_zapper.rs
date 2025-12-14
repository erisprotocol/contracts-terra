use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Uint128};

#[cw_serde]
pub enum AssetInfoBase<T> {
    Native(String),
    Cw20(T),
}
pub type AssetInfoUnchecked = AssetInfoBase<String>;
pub type AssetInfo = AssetInfoBase<Addr>;

#[cw_serde]
pub struct AssetBase<T> {
    /// Specifies the asset's type (CW20 or native)
    pub info: AssetInfoBase<T>,
    /// Specifies the asset's amount
    pub amount: Uint128,
}

pub type AssetUnchecked = AssetBase<String>;
pub type Asset = AssetBase<Addr>;

#[cw_serde]
pub struct InstantiateMsg {
    pub global_config_addr: String,
    pub center_asset_infos: Vec<AssetInfoUnchecked>,
}

#[cw_serde]
pub enum ExecuteMsg {
    CreateLp {
        stage: StageType,
        assets: Vec<AssetInfo>,
        min_received: Option<Uint128>,
        post_action: Option<PostActionCreate>,
    },
    WithdrawLp {
        stage: StageType,
        min_received: Option<Vec<Asset>>,
        post_action: Option<PostActionWithdraw>,
    },

    /// Swaps a number of assets to a single result
    Swap {
        /// LP into which the assets should be compounded into
        into: AssetInfoUnchecked,
        /// List of reward asset send to compound
        assets: Vec<AssetInfoUnchecked>,
        min_received: Option<Uint128>,
        /// Receiver address for LP token
        receiver: Option<String>,
    },

    Zap {
        into: AssetInfoUnchecked,
        assets: Vec<AssetInfoUnchecked>,
        min_received: Option<Uint128>,
        post_action: Option<PostActionCreate>,
    },

    UpdateConfig {
        insert_routes: Option<Vec<RouteInit>>,
        delete_routes: Option<Vec<RouteDelete>>,
        update_centers: Option<Vec<AssetInfoUnchecked>>,
        register_single_direction: Option<bool>,
    },
}

#[cw_serde]
pub struct RouteInit {
    pub routes: Vec<Stage>,
}

#[cw_serde]
pub struct RouteDelete {
    pub from: AssetInfo,
    pub to: AssetInfo,
    pub both: Option<bool>,
}

#[cw_serde]
pub enum PostActionCreate {
    Stake {
        asset_staking: Addr,
        receiver: Option<String>,
    },
    LiquidStake {
        compounder: Addr,
        gauge: String,
        receiver: Option<String>,
    },
    SendResult {
        receiver: Option<String>,
    },
    ExecuteResult {
        contract: String,
        msg: Binary,
    },
}

#[cw_serde]
pub enum PostActionWithdraw {
    SwapTo {
        asset: AssetInfo,
        min_received: Option<Uint128>,
        receiver: Option<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    Config {},

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
        from: AssetInfoUnchecked,
        to: AssetInfoUnchecked,
    },
}

#[cw_serde]
pub struct Config {
    pub global_config_addr: Addr,
    #[serde(default)]
    pub center_asset_infos: Vec<AssetInfo>,
}

#[cw_serde]
pub struct SupportsSwapResponse {
    pub suppored: bool,
}

#[cw_serde]
pub struct RouteResponseItem {
    pub key: (AssetInfo, AssetInfo),
    pub stages: Vec<Stage>,
}

#[cw_serde]
pub struct Stage {
    pub from: AssetInfo,
    pub to: AssetInfo,
    pub stage_type: StageType,
}

#[cw_serde]
pub enum StageType {
    WhiteWhale {
        pair: Addr,
    },
    Astroport {
        pair: Addr,
    },
}

#[cw_serde]
pub struct MigrateMsg {}
