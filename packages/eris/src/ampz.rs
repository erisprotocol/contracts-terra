use std::fmt;

use astroport::asset::{Asset, AssetInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_binary, Addr, Api, Coin, CosmosMsg, StdError, StdResult, Uint128, WasmMsg};

use crate::{adapters::generator::Generator, helpers::bps::BasicPoints};

/// This structure describes the basic settings for creating a contract.
#[cw_serde]
pub struct InstantiateMsg {
    /// Contract owner
    pub owner: String,
    /// The controller address to execute compound
    pub controller: String,
    /// Hub contract address
    pub hub: String,
    /// Arb vault contract address
    pub arb_vault: String,
    /// Amp compounder farms
    pub farms: Vec<String>,

    pub zapper: String,
    pub astroport: AstroportConfig<String>,
    pub whitewhale: WhiteWhaleConfig<String>,
    pub capapult: CapapultConfig<String>,
    pub fee: FeeConfig<String>,
}

#[cw_serde]
pub enum Source {
    Claim,
    ClaimContract {
        claim_type: ClaimType,
    },
    AstroRewards {
        lps: Vec<String>,
    },
    WhiteWhaleRewards {
        lps: Vec<String>,
    },
    Wallet {
        over: Asset,
        max_amount: Option<Uint128>,
    },
}

#[cw_serde]
pub enum ClaimType {
    WhiteWhaleRewards,
}
impl fmt::Display for ClaimType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ClaimType::WhiteWhaleRewards => write!(f, "WhiteWhaleRewards"),
        }
    }
}

#[cw_serde]
pub struct VestingInfo {
    pub periods: Vec<VestingPeriod>,
    pub start_time: u64,
    pub end_time: u64,
}

#[cw_serde]
pub struct VestingPeriod {
    pub length: u64,
    pub amount: Vec<Coin>,
}

#[cw_serde]
pub struct Execution {
    pub user: String,
    pub source: Source,
    pub destination: DestinationState,
    pub schedule: Schedule,
}

impl Source {
    pub fn try_get_uniq_key(&self) -> Option<String> {
        match self {
            Source::Claim => Some("claim".to_string()),
            Source::ClaimContract {
                claim_type,
            } => Some(format!("claim_{0}", claim_type)),
            Source::AstroRewards {
                ..
            } => Some("astro_rewards".to_string()),
            Source::WhiteWhaleRewards {
                ..
            } => Some("whitewhale_rewards".to_string()),
            Source::Wallet {
                ..
            } => {
                // wallet is allowed to be defined multiple times
                None
            },
        }
    }
}

#[cw_serde]
pub struct Schedule {
    pub start: Option<u64>,
    pub interval_s: u64,
}

#[cw_serde]
pub struct AstroportConfig<T> {
    pub generator: T,
    pub coins: Vec<AssetInfo>,
}

impl AstroportConfig<String> {
    pub fn validate(self, api: &dyn Api) -> StdResult<AstroportConfig<Generator>> {
        Ok(AstroportConfig {
            generator: Generator(api.addr_validate(&self.generator)?),
            coins: self.coins,
        })
    }
}
#[cw_serde]
pub struct WhiteWhaleConfig<T> {
    pub fee_distributor: T,
    pub coins: Vec<AssetInfo>,
    pub incentive_factory_addr: T,
    pub lp_tokens: Vec<T>,
}

impl WhiteWhaleConfig<String> {
    pub fn validate(self, api: &dyn Api) -> StdResult<WhiteWhaleConfig<Addr>> {
        Ok(WhiteWhaleConfig {
            fee_distributor: api.addr_validate(&self.fee_distributor)?,
            coins: self.coins,
            incentive_factory_addr: api.addr_validate(&self.incentive_factory_addr)?,
            lp_tokens: self
                .lp_tokens
                .into_iter()
                .map(|lp| api.addr_validate(&lp))
                .collect::<StdResult<Vec<_>>>()?,
        })
    }
}

#[cw_serde]
pub struct CapapultConfig<T> {
    pub market: T,
    pub overseer: T,
    pub stable_cw: T,
    pub custody: T,
}

impl CapapultConfig<String> {
    pub fn validate(self, api: &dyn Api) -> StdResult<CapapultConfig<Addr>> {
        Ok(CapapultConfig {
            market: api.addr_validate(&self.market)?,
            overseer: api.addr_validate(&self.overseer)?,
            custody: api.addr_validate(&self.custody)?,
            stable_cw: api.addr_validate(&self.stable_cw)?,
        })
    }
}

/// This structure describes the execute messages available in the contract.
#[cw_serde]
pub enum ExecuteMsg {
    Execute {
        id: Uint128,
    },

    // being executed via authz
    Deposit {
        assets: Vec<Asset>,
    },

    AddExecution {
        overwrite: bool,
        execution: Execution,
    },
    RemoveExecutions {
        ids: Option<Vec<Uint128>>,
    },

    /// The callback of type [`CallbackMsg`]
    Callback(CallbackWrapper),

    /// Transfer ownership to another account; will not take effect unless the new owner accepts
    TransferOwnership {
        new_owner: String,
    },
    /// Remove the ownership transfer proposal
    DropOwnershipProposal {},
    /// Accept an ownership transfer
    AcceptOwnership {},

    UpdateConfig {
        add_farms: Option<Vec<String>>,
        remove_farms: Option<Vec<String>>,
        controller: Option<String>,
        zapper: Option<String>,
        hub: Option<String>,
        arb_vault: Option<String>,
        astroport: Option<AstroportConfig<String>>,
        capapult: Option<CapapultConfig<String>>,
        fee: Option<FeeConfig<String>>,
    },
}

#[cw_serde]
pub struct FeeConfig<T> {
    pub fee_bps: BasicPoints,
    pub operator_bps: BasicPoints,
    pub receiver: T,
}

impl FeeConfig<String> {
    pub fn validate(self, api: &dyn Api) -> StdResult<FeeConfig<Addr>> {
        if self.fee_bps.u16() > 500 {
            return Err(StdError::generic_err("max fee is 5 %"));
        }
        if self.operator_bps.u16() > 500 {
            return Err(StdError::generic_err("max operator fee is 5 %"));
        }

        Ok(FeeConfig {
            fee_bps: self.fee_bps,
            operator_bps: self.operator_bps,
            receiver: api.addr_validate(&self.receiver)?,
        })
    }
}

// #[cw_serde]
// pub struct TipConfig {
//     pub pay_tips_in: AssetInfo,
//     pub amplifier: Uint128,
//     pub per_farm: Uint128,
// }

#[cw_serde]
pub struct CallbackWrapper {
    pub id: u128,
    pub user: Addr,
    pub message: CallbackMsg,
}

#[cw_serde]
pub enum DestinationState {
    DepositAmplifier {
        #[serde(default)]
        receiver: Option<Addr>,
    },
    DepositArbVault {
        receiver: Option<Addr>,
    },
    DepositFarm {
        farm: String,
        #[serde(default)]
        receiver: Option<Addr>,
    },
    SwapTo {
        asset_info: AssetInfo,
        #[serde(default)]
        receiver: Option<Addr>,
    },
    DepositCollateral {
        market: DepositMarket,
    },
    Repay {
        market: RepayMarket,
    },
    DepositLiquidity {
        lp_token: String,
        dex: DepositLiquidity,
    },
}

#[cw_serde]
pub enum DepositLiquidity {
    WhiteWhale {
        lock_up: Option<u64>,
    },
}

#[cw_serde]
pub enum DestinationRuntime {
    DepositAmplifier {
        receiver: Option<Addr>,
    },
    DepositArbVault {
        receiver: Option<Addr>,
    },
    DepositFarm {
        asset_infos: Vec<AssetInfo>,
        farm: String,
        receiver: Option<Addr>,
    },
    SendSwapResultToUser {
        asset_info: AssetInfo,
        receiver: Option<Addr>,
    },
    DepositCollateral {
        market: DepositMarket,
    },
    Repay {
        market: RepayMarket,
    },
    DepositLiquidity {
        asset_infos: Vec<AssetInfo>,
        lp_token: String,
        dex: DepositLiquidity,
    },
}

#[cw_serde]
pub enum DepositMarket {
    Capapult {
        // specifies which asset to deposit into capapult
        asset_info: AssetInfo,
    },
}

#[cw_serde]
pub enum RepayMarket {
    Capapult,
}

/// This structure describes the callback messages of the contract.
#[cw_serde]
pub enum CallbackMsg {
    AuthzDeposit {
        user_balance_start: Vec<Asset>,
        max_amount: Option<Vec<Asset>>,
    },

    AuthzLockWwLp {
        lp_balance: Asset,
        unbonding_duration: u64,
    },

    Swap {
        asset_infos: Vec<AssetInfo>,
        into: AssetInfo,
    },

    FinishExecution {
        destination: DestinationRuntime,
        executor: Addr,
    },
}

// Modified from
// https://github.com/CosmWasm/cw-plus/blob/v0.8.0/packages/cw20/src/receiver.rs#L23
impl CallbackMsg {
    pub fn into_cosmos_msg(
        &self,
        contract_addr: &Addr,
        id: u128,
        user: &Addr,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: String::from(contract_addr),
            msg: to_binary(&ExecuteMsg::Callback(self.into_callback_wrapper(id, user)))?,
            funds: vec![],
        }))
    }

    pub fn into_callback_wrapper(&self, id: u128, user: &Addr) -> CallbackWrapper {
        CallbackWrapper {
            id,
            user: user.clone(),
            message: self.clone(),
        }
    }
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// The contract's configurations. Response: `ConfigResponse`
    #[returns(ConfigResponse)]
    Config {},

    #[returns(StateResponse)]
    State {},

    #[returns(UserInfoResponse)]
    UserInfo {
        user: String,
    },

    #[returns(ExecutionsResponse)]
    Executions {
        start_after: Option<Uint128>,
        limit: Option<u32>,
    },

    #[returns(ExecutionsScheduleResponse)]
    ExecutionsSchedule {
        start_after: Option<Uint128>,
        limit: Option<u32>,
    },

    #[returns(ExecutionResponse)]
    Execution {
        id: Uint128,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    /// Hub contract
    pub hub: String,
    /// Arb Vault contract
    pub arb_vault: String,
    /// Farms
    pub farms: Vec<String>,

    /// Account who can call certain privileged functions
    pub owner: String,
    /// Pending ownership transfer, awaiting acceptance by the new owner
    pub new_owner: Option<String>,

    pub controller: String,

    pub zapper: String,

    pub astroport: AstroportConfig<String>,

    pub capapult: CapapultConfig<String>,

    pub fee: FeeConfig<String>,
}

#[cw_serde]
pub struct StateResponse {
    pub next_id: Uint128,
}

#[cw_serde]
pub struct UserInfoResponse {
    pub executions: Vec<ExecutionDetail>,
}

#[cw_serde]
pub struct ExecutionsResponse {
    pub executions: Vec<(Uint128, Execution)>,
}

#[cw_serde]
pub struct ExecutionsScheduleResponse {
    // (id, last_execution, interval_s)
    pub executions: Vec<ExecutionSchedule>,
}

#[cw_serde]
pub struct ExecutionResponse {
    pub detail: ExecutionDetail,
}

#[cw_serde]
pub struct ExecutionDetail {
    pub id: Uint128,
    pub execution: Execution,
    pub last_execution: u64,
    pub can_execute: bool,
}

#[cw_serde]
pub struct ExecutionSchedule {
    pub id: Uint128,
    pub last_execution: u64,
    pub interval_s: u64,
}

#[cw_serde]
pub struct MigrateMsg {}
