use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, Decimal, Empty, Response, StdError, StdResult, Uint128,
    WasmMsg,
};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LiquidStakingType {
    Eris,
    Stader,
    Steak,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ExtractConfig {
    /// Address where extracted yield should be deposited
    pub yield_extract_addr: Addr,
    /// Percentage of yield that should be extracted (between 0 and 1)
    pub yield_extract_p: Decimal, // "1 is 100%, 0.05 is 5%"
    /// Hub contract
    pub hub_contract: Addr,
    /// defines how to interact with the hub_contract for reading the exchange_rate
    pub interface: LiquidStakingType,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Account who can call certain privileged functions
    pub owner: String,

    /// Code ID of the CW20 token contract for the pool token
    pub cw20_code_id: u64,
    /// Symbol of the yield extractor pool token
    pub symbol: String,
    /// Number of decimals of the yield extractor pool token
    pub decimals: u8,
    /// Name of yield extractor pool token
    pub name: String,

    /// Hub contract
    pub hub_contract: String,
    // Stake token
    pub stake_token: String,

    /// defines how to interact with the hub_contract for reading the exchange_rate
    pub interface: LiquidStakingType,
    /// Address where extracted yield should be deposited
    pub yield_extract_addr: String,
    /// Percentage of yield that should be extracted (between 0 and 1)
    pub yield_extract_p: Decimal, // "1 is 100%, 0.05 is 5%"
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Implements the Cw20 receiver interface
    Receive(Cw20ReceiveMsg),

    /// Claim yield extracted rewards
    Harvest {},

    /// Updates the fee config,
    UpdateConfig {
        /// Contract address where fees are sent
        yield_extract_addr: Option<String>,
        /// Fees that are being applied during reinvest of staking rewards
        yield_extract_p: Option<Decimal>, // "1 is 100%, 0.05 is 5%"
    },

    /// Transfer ownership to another account; will not take effect unless the new owner accepts
    TransferOwnership {
        new_owner: String,
    },
    /// Accept an ownership transfer
    AcceptOwnership {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveMsg {
    /// Deposit cw20 ampLuna into the vault
    Deposit {},

    Withdraw {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// The contract's configurations. Response: `ConfigResponse`
    Config {},
    /// The contract's current state. Response: `StateResponse`
    State {},
    // Returns information about the share value [`ShareResponse`].
    Share {
        addr: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    /// Hub contract
    pub hub_contract: String,
    /// defines how to interact with the hub_contract for reading the exchange_rate
    pub interface: LiquidStakingType,
    /// Address of the lp token
    pub lp_token: String,
    /// Address of the stake token
    pub stake_token: String,

    /// Account who can call certain privileged functions
    pub owner: String,
    /// Pending ownership transfer, awaiting acceptance by the new owner
    pub new_owner: Option<String>,

    /// Address where extracted yield should be deposited
    pub yield_extract_addr: String,
    /// Percentage of yield that should be extracted (between 0 and 1)
    pub yield_extract_p: Decimal, // "1 is 100%, 0.05 is 5%"
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateResponse {
    /// Total supply to the lp token
    pub total_lp: Uint128,
    /// Total amount of uluna staked (bonded)
    pub total_lsd: Uint128,
    // Amount of lsd to be harvestable
    pub harvestable: Uint128,

    pub total_harvest: Uint128,

    /// The exchange rate between ustake and uluna, in terms of uluna per ustake
    pub exchange_rate_lp_lsd: Decimal,
    /// The exchange rate between the liquid staking derivate and uluna
    pub exchange_rate_lsd_uluna: Decimal,
    // Total amount of uluna within the contract (lsd * exchange_rate_lsd_uluna)
    pub tvl_uluna: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ShareResponse {
    pub received_asset: Uint128,
    pub share: Uint128,
    pub total_lp: Uint128,
}

pub type MigrateMsg = Empty;
