use astroport::asset::Asset;
use cosmwasm_std::{to_binary, Addr, CosmosMsg, Decimal, StdResult, Uint128, WasmMsg};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// This structure describes the parameters for creating a contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// The owner address
    pub owner: String,
    /// The LP staking generator contract address
    pub staking_contract: String,
    /// The compound proxy contract address
    pub compound_proxy: String,
    /// The controller address to execute compound
    pub controller: String,
    /// The performance fee
    pub fee: Decimal,
    /// The fee collector contract address
    pub fee_collector: String,
    /// The LP token contract address
    pub liquidity_token: String,
    /// the base reward token contract address
    pub base_reward_token: String,

    /// Information about the amp[LP] Token for pool shares.
    pub amp_lp: TokenInit,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenInit {
    /// Code Id of the amp[LP] token
    pub cw20_code_id: u64,
    /// Name of the liquid staking token
    pub name: String,
    /// Symbol of the liquid staking token
    pub symbol: String,
    /// Number of decimals of the liquid staking token
    pub decimals: u8,
}

/// This structure describes the execute messages available in the contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Receives a message of type [`Cw20ReceiveMsg`]
    Receive(Cw20ReceiveMsg),
    /// Update contract config
    UpdateConfig {
        /// The compound proxy contract address
        compound_proxy: Option<String>,
        /// The controller address
        controller: Option<String>,
        /// The performance fee
        fee: Option<Decimal>,
        /// The fee collector contract address
        fee_collector: Option<String>,
    },
    /// Compound LP rewards
    Compound {
        /// The minimum expected amount of LP token
        minimum_receive: Option<Uint128>,
        /// Slippage tolerance when providing LP
        slippage_tolerance: Option<Decimal>,
    },
    /// Bond asset with optimal swap
    BondAssets {
        /// The list of asset to bond
        assets: Vec<Asset>,
        /// The minimum expected amount of LP token
        minimum_receive: Option<Uint128>,
        /// The flag to skip optimal swap
        no_swap: Option<bool>,
        /// Slippage tolerance when providing LP
        slippage_tolerance: Option<Decimal>,
    },
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
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CallbackMsg {
    Stake {
        /// The previous LP balance in the contract
        prev_balance: Uint128,
        /// The minimum expected amount of LP token
        minimum_receive: Option<Uint128>,
    },
    BondTo {
        /// The address to bond LP
        to: Addr,
        /// The previous LP balance in the contract
        prev_balance: Uint128,
        /// The minimum expected amount of LP token
        minimum_receive: Option<Uint128>,
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

/// This structure describes custom hooks for the CW20.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    // Bond LP token
    Bond {
        staker_addr: Option<String>,
    },

    // Unbond LP token
    Unbond {
        receiver: Option<String>,
    },
}

/// This structure describes query messages available in the contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Returns the contract config
    Config {},
    /// Returns the deposited balances
    UserInfo {
        addr: String,
    },
    /// Returns the global state
    State {
        addr: Option<String>,
    },
}

/// This structure holds the parameters for reward info query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserInfoResponse {
    /// The LP token amount bonded
    pub user_lp_amount: Uint128,
    /// The share of total LP token bonded
    pub user_amp_lp_amount: Uint128,
    /// Total lp balance of pool
    pub total_lp: Uint128,
    // total amount of minted amp[LP] tokens (= total shares)
    pub total_amp_lp: Uint128,
}

/// This structure describes a migration message.
/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    // Addr of the underlying lp token
    pub lp_token: Addr,
    // Addr of the amp[LP] token
    pub amp_lp_token: Addr,

    pub owner: Addr,
    pub staking_contract: Addr,
    pub compound_proxy: Addr,
    pub controller: Addr,
    pub fee: Decimal,
    pub fee_collector: Addr,
    pub base_reward_token: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateResponse {
    // total amount of underlying LP managed in the pool.
    pub total_lp: Uint128,
    // total amount of minted amp[LP] tokens
    pub total_amp_lp: Uint128,
    /// The exchange rate between amp[LP] and LP, in terms of LP per amp[LP]
    pub exchange_rate: Decimal,

    pub pair_contract: Addr,

    pub locked_assets: Vec<Asset>,

    pub user_info: Option<UserInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserInfo {
    /// The LP token amount bonded
    pub user_lp_amount: Uint128,
    /// The share of total LP token bonded
    pub user_amp_lp_amount: Uint128,
}
