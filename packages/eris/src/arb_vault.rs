use astroport::asset::{Asset, AssetInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::{
    to_binary, Addr, Api, Binary, CosmosMsg, Decimal, StdError, StdResult, Uint128, WasmMsg,
};
use cw20::Cw20ReceiveMsg;

// /// The default swap slippage
// pub const DEFAULT_SLIPPAGE: &str = "0.005";
// /// The maximum allowed swap slippage
// pub const MAX_ALLOWED_SLIPPAGE: &str = "0.5";

#[cw_serde]
pub struct InstantiateMsg {
    // Code id for LP Token
    pub cw20_code_id: u64,
    /// Name of the liquid staking token
    pub name: String,
    /// Symbol of the liquid staking token
    pub symbol: String,
    /// Number of decimals of the liquid staking token
    pub decimals: u8,

    pub owner: String,

    // Used base token
    pub utoken: String,
    // execution threshold
    pub utilization_method: UtilizationMethod,
    // min unbond time 21+3 * 24 * 60 * 60
    pub unbond_time_s: u64,
    // config for lsds
    pub lsds: Vec<LsdConfig<String>>,

    // config for the fees
    pub fee_config: FeeConfig<String>,

    // executors that are allowed to execute arbitrage
    pub whitelist: Vec<String>,
}

#[cw_serde]
pub struct LsdConfig<T> {
    pub disabled: bool,
    pub name: String,
    pub lsd_type: LsdType<T>,
}

impl LsdConfig<String> {
    pub fn validate(self, api: &dyn Api) -> StdResult<LsdConfig<Addr>> {
        Ok(LsdConfig {
            disabled: self.disabled,
            name: self.name,
            lsd_type: match self.lsd_type {
                LsdType::Eris {
                    addr,
                    cw20,
                } => LsdType::Eris {
                    addr: api.addr_validate(&addr)?,
                    cw20: api.addr_validate(&cw20)?,
                },
                LsdType::Backbone {
                    addr,
                    cw20,
                } => LsdType::Backbone {
                    addr: api.addr_validate(&addr)?,
                    cw20: api.addr_validate(&cw20)?,
                },
                LsdType::Stader {
                    addr,
                    cw20,
                } => LsdType::Stader {
                    addr: api.addr_validate(&addr)?,
                    cw20: api.addr_validate(&cw20)?,
                },
                LsdType::Prism {
                    addr,
                    cw20,
                } => LsdType::Prism {
                    addr: api.addr_validate(&addr)?,
                    cw20: api.addr_validate(&cw20)?,
                },
            },
        })
    }
}

#[cw_serde]
pub enum LsdType<T> {
    Eris {
        addr: T,
        cw20: T,
    },
    Backbone {
        addr: T,
        cw20: T,
    },
    Stader {
        addr: T,
        cw20: T,
    },
    Prism {
        addr: T,
        cw20: T,
    },
}

impl LsdType<String> {
    pub fn to_uniq_key(&self) -> String {
        match self {
            LsdType::Eris {
                addr,
                ..
            } => format!("eris_{0}", addr),
            LsdType::Backbone {
                addr,
                ..
            } => format!("backbone_{0}", addr),
            LsdType::Stader {
                addr,
                ..
            } => format!("stader_{0}", addr),
            LsdType::Prism {
                addr,
                ..
            } => format!("prism_{0}", addr),
        }
    }
}

#[cw_serde]
pub enum UtilizationMethod {
    Steps(Vec<(Decimal, Decimal)>),
}

#[cw_serde]
pub struct ExecuteSubMsg {
    pub contract_addr: Option<String>,
    pub msg: Binary,
    pub funds_amount: Uint128,
}

/// This structure describes the execute messages available in the contract.
#[cw_serde]
pub enum ExecuteMsg {
    // User action: Receive to queue funds for withdraw
    Receive(Cw20ReceiveMsg),
    // User action: Provide liquidity to the pool and specify who will receive the pool token.
    Deposit {
        asset: Asset,
        receiver: Option<String>,
    },

    // User action: Withdraw all unbonded funds
    WithdrawUnbonded {},
    // User action: Withdraw any unbond item immediate if possible
    WithdrawImmediate {
        id: u64,
    },

    // /// Swap is allowed between the [TOKEN] and the arb[TOKEN]
    // Swap {
    //     offer_asset: Asset,
    //     ask_asset_info: Option<AssetInfo>,
    //     belief_price: Option<Decimal>,
    //     max_spread: Option<Decimal>,
    //     to: Option<String>,
    // },

    // Admin User: Update config
    UpdateConfig {
        utilization_method: Option<UtilizationMethod>,
        unbond_time_s: Option<u64>,

        // insert an lsd
        insert_lsd: Option<LsdConfig<String>>,
        // disable LSD
        disable_lsd: Option<String>,
        // removes a LSD (only when nothing is unbonding and withdrawable)
        remove_lsd: Option<String>,

        fee_config: Option<FeeConfig<String>>,
        set_whitelist: Option<Vec<String>>,
        // opens up executions so anyone can execute.
        remove_whitelist: Option<bool>,
    },

    // Bot: Execute arbitrage
    ExecuteArbitrage {
        // specify what kind of action should be executed
        msg: ExecuteSubMsg,
        // what is the result token for unbonding action
        result_token: AssetInfo,
        // Specify the goal profit: 0.01 -> 1 %
        wanted_profit: Decimal,
    },

    // Bot: Withdraw unbonded liquidity from liquid staking providers
    WithdrawFromLiquidStaking {
        // specify which adapters should be withdrawn
        names: Option<Vec<String>>,
    },
    // Bot: Start unbonding liquidity from liquid staking providers
    UnbondFromLiquidStaking {
        // specify which adapters should be withdrawn
        names: Option<Vec<String>>,
    },

    // Internal: Asserts that the execution was a success and the wanted_profit reached.
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

    /// The callback of type [`CallbackMsg`]
    Callback(CallbackMsg),
}

/// This structure describes the callback messages of the contract.
#[cw_serde]
pub enum CallbackMsg {
    AssertResult {
        result_token: AssetInfo,
        wanted_profit: Decimal,
    },
}

impl CallbackMsg {
    pub fn into_cosmos_msg(&self, contract_addr: &Addr) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
            msg: to_binary(&ExecuteMsg::Callback(self.clone()))?,
            funds: vec![],
        }))
    }
}

/// This structure describes a CW20 hook message.
#[cw_serde]

pub enum Cw20HookMsg {
    /// Withdraw liquidity from the pool
    Unbond {
        immediate: Option<bool>,
    },
    // /// Swap is allowed between arb[TOKEN] and allowed swap LSDs e.g. amp[TOKEN]
    // Swap {
    //     ask_asset_info: Option<AssetInfo>,
    //     belief_price: Option<Decimal>,
    //     max_spread: Option<Decimal>,
    //     to: Option<String>,
    // },
}

/// This structure describes the query messages available in the contract.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns contract configuration settings in a custom [`ConfigResponse`] structure.
    #[returns(ConfigResponse)]
    Config {},

    #[returns(StateResponse)]
    State {
        details: Option<bool>,
    },

    /// Returns information about the share value
    #[returns(UserInfoResponse)]
    UserInfo {
        address: String,
    },

    /// Query available funds for specified profit goal.
    #[returns(TakeableResponse)]
    Takeable {
        wanted_profit: Option<Decimal>,
    },

    /// Query user funds currently unbonding
    #[returns(UnbondRequestsResponse)]
    UnbondRequests {
        address: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },

    #[returns(ExchangeRatesResponse)]
    ExchangeRates {
        // start after the provided timestamp in days
        start_after_d: Option<u64>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct UnbondItem {
    pub start_time: u64,
    pub released: bool,
    pub release_time: u64,
    pub amount_asset: Uint128,
    pub withdraw_protocol_fee: Uint128,
    pub withdraw_pool_fee: Uint128,
    pub id: u64,
}

#[cw_serde]
pub struct UnbondRequestsResponse {
    pub requests: Vec<UnbondItem>,
}

#[cw_serde]
pub struct WithdrawableResponse {
    pub withdrawable: Uint128,
}

#[cw_serde]
pub struct TakeableResponse {
    pub takeable: Option<Uint128>,
    pub steps: Vec<(Decimal, Uint128)>,
}

/// This struct is used to return a query result with the general contract configuration.
#[cw_serde]
pub struct ConfigResponse {
    pub config: Config<Addr>,
    pub fee_config: FeeConfig<Addr>,
    pub owner: Addr,
}

/// ## Description
/// This structure stores the main config parameters for a constant product pair contract.
#[cw_serde]
pub struct Config<T> {
    pub utoken: String,
    pub utilization_method: UtilizationMethod,
    pub unbond_time_s: u64,
    pub lp_addr: T,
    pub lsds: Vec<LsdConfig<T>>,
}

pub type ValidatedConfig = Config<Addr>;

#[cw_serde]
pub struct FeeConfig<T> {
    pub protocol_fee_contract: T,
    pub protocol_performance_fee: Decimal,
    pub protocol_withdraw_fee: Decimal,
    pub immediate_withdraw_fee: Decimal,
}
pub type ValidatedFeeConfig = FeeConfig<Addr>;

impl FeeConfig<String> {
    pub fn validate(self, api: &dyn Api) -> StdResult<FeeConfig<Addr>> {
        if self.protocol_performance_fee > Decimal::percent(20) {
            return Err(StdError::generic_err("Performance fee too high"));
        }

        if self.protocol_withdraw_fee > Decimal::percent(5) {
            return Err(StdError::generic_err("Protocol withdraw fee too high"));
        }

        if self.immediate_withdraw_fee > Decimal::percent(10) {
            return Err(StdError::generic_err("Immediate withdraw fee too high"));
        }

        Ok(FeeConfig {
            protocol_fee_contract: api.addr_validate(&self.protocol_fee_contract)?,
            protocol_performance_fee: self.protocol_performance_fee,
            protocol_withdraw_fee: self.protocol_withdraw_fee,
            immediate_withdraw_fee: self.immediate_withdraw_fee,
        })
    }
}

#[cw_serde]
pub struct StateResponse {
    pub exchange_rate: Decimal,
    pub total_lp_supply: Uint128,
    pub balances: BalancesOptionalDetails,

    pub details: Option<StateDetails>,
}

#[cw_serde]
pub struct UserDetails {}

#[cw_serde]
pub struct StateDetails {
    pub takeable_steps: Vec<(Decimal, Uint128)>,
}

#[cw_serde]
pub struct Balances<T> {
    // total locked value (utoken) in the contract (vault_available + lsd_unbonding + lsd_withdrawable + lsd_xvalue)
    pub tvl_utoken: Uint128,
    // total value used for arbitrage (tvl_utoken - locked_user_withdrawls)
    pub vault_total: Uint128,
    // funds available in the contract
    pub vault_available: Uint128,
    // funds that can be used by the arbitrage (vault_available - locked_user_withdrawls)
    pub vault_takeable: Uint128,
    // funds that are currently being withdrawn
    pub locked_user_withdrawls: Uint128,
    // amount that is currently unbonding
    pub lsd_unbonding: Uint128,
    // amount that is currently withdrawable
    pub lsd_withdrawable: Uint128,
    // amount that is currently in the contract as xtokens denominated in the utoken(amp[TOKEN], b[TOKEN], etc.)
    pub lsd_xvalue: Uint128,

    pub details: T,
}

pub type BalancesDetails = Balances<Vec<ClaimBalance>>;
pub type BalancesOptionalDetails = Balances<Option<Vec<ClaimBalance>>>;

#[cw_serde]
pub struct UserInfoResponse {
    pub utoken_amount: Uint128,
    pub lp_amount: Uint128,
}

#[cw_serde]
pub struct ClaimBalance {
    pub name: String,
    pub withdrawable: Uint128,
    pub unbonding: Uint128,
    pub xbalance: Uint128,
    pub xfactor: Decimal,
}

#[cw_serde]
pub struct ExchangeRatesResponse {
    pub exchange_rates: Vec<(u64, ExchangeHistory)>,
    // APR normalized per DAY
    pub apr: Option<Decimal>,
}

#[cw_serde]
pub struct ExchangeHistory {
    pub exchange_rate: Decimal,
    pub time_s: u64,
}

/// This structure describes a migration message.
/// We currently take no arguments for migrations.
#[cw_serde]
pub struct MigrateMsg {}
