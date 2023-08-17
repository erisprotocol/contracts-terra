use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Empty, Uint128};
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub enum LiquidStakingType {
    Eris,
    Stader,
    Steak,
}

#[cw_serde]
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

#[cw_serde]
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
    /// Label for the token
    pub label: String,

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

#[cw_serde]
pub enum ExecuteMsg {
    /// Implements the Cw20 receiver interface
    Receive(Cw20ReceiveMsg),

    /// Claim yield extracted rewards
    Harvest {},

    /// Updates the fee config,
    UpdateConfig {
        /// Contract address where fees are sent
        yield_extract_addr: Option<String>,
    },

    /// Transfer ownership to another account; will not take effect unless the new owner accepts
    TransferOwnership {
        new_owner: String,
    },
    /// Accept an ownership transfer
    AcceptOwnership {},
}

#[cw_serde]
pub enum ReceiveMsg {
    /// Deposit cw20 amp[Token] into the vault
    Deposit {},

    Withdraw {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// The contract's configurations. Response: `ConfigResponse`
    #[returns(ConfigResponse)]
    Config {},
    /// The contract's current state. Response: `StateResponse`
    #[returns(StateResponse)]
    State {
        // if addr is provided, will also return the state for the addr
        addr: Option<String>,
    },
    // Returns information about the share value [`ShareResponse`].
    #[returns(ShareResponse)]
    Share {
        addr: Option<String>,
    },
}

#[cw_serde]
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

#[cw_serde]
pub struct StateResponse {
    /// Total supply to the lp token
    pub total_lp: Uint128,
    /// Total amount of utoken staked (bonded)
    pub stake_balance: Uint128,
    // Amount of lsd to be harvestable
    pub stake_extracted: Uint128,

    // Total stake harvested
    pub stake_harvested: Uint128,

    // stake_balance - stake_extracted
    pub stake_available: Uint128,

    /// The exchange rate between ustake and utoken, in terms of utoken per ustake
    pub exchange_rate_lp_stake: Decimal,
    /// The exchange rate between the liquid staking derivate and utoken
    pub exchange_rate_stake_utoken: Decimal,
    // Total amount of utoken within the contract (stake_balance * exchange_rate_stake_utoken)
    pub tvl_utoken: Uint128,

    // amount of LP shares the provided addr holds
    pub user_share: Option<Uint128>,
    // amount of assets the user would get (stake)
    pub user_received_asset: Option<Uint128>,
}

#[cw_serde]
pub struct ShareResponse {
    pub received_asset: Uint128,
    pub share: Uint128,
    pub total_lp: Uint128,
}

pub type MigrateMsg = Empty;
