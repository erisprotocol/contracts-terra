use cosmwasm_std::{
    to_binary, Addr, Api, Coin, CosmosMsg, Decimal, Empty, QuerierWrapper, StdResult, Timestamp,
    Uint128, WasmMsg,
};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::helper::{addr_opt_validate, addr_validate_to_lower};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DelegationStrategy<T = String> {
    /// all validators receive the same delegation.
    Uniform,
    /// validators receive delegations based on community voting + merit points
    Gauges {
        /// gauges based on vAmp voting
        amp_gauges: T,
        /// gauges based on eris merit points
        emp_gauges: Option<T>,
        /// weight between amp and emp gauges between 0 and 1
        amp_factor_bps: u16,
        /// min amount of delegation needed
        min_delegation_bps: u16,
        /// max amount of delegation needed
        max_delegation_bps: u16,
        /// count of validators that should receive delegations
        validator_count: u8,
    },
}

impl DelegationStrategy<String> {
    pub fn validate(self, api: &dyn Api) -> StdResult<DelegationStrategy<Addr>> {
        let result = match self {
            DelegationStrategy::Uniform {} => DelegationStrategy::Uniform {},
            DelegationStrategy::Gauges {
                amp_gauges,
                emp_gauges,
                amp_factor_bps: amp_factor,
                min_delegation_bps,
                validator_count,
                max_delegation_bps,
            } => DelegationStrategy::Gauges {
                amp_gauges: addr_validate_to_lower(api, amp_gauges)?,
                emp_gauges: addr_opt_validate(api, &emp_gauges)?,
                amp_factor_bps: amp_factor,
                min_delegation_bps,
                validator_count,
                max_delegation_bps,
            },
        };
        Ok(result)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    /// Code ID of the CW20 token contract
    pub cw20_code_id: u64,
    /// Account who can call certain privileged functions
    pub owner: String,
    /// Name of the liquid staking token
    pub name: String,
    /// Symbol of the liquid staking token
    pub symbol: String,
    /// Number of decimals of the liquid staking token
    pub decimals: u8,
    /// How often the unbonding queue is to be executed, in seconds
    pub epoch_period: u64,
    /// The staking module's unbonding time, in seconds
    pub unbond_period: u64,
    /// Initial set of validators who will receive the delegations
    pub validators: Vec<String>,

    /// Contract address where fees are sent
    pub protocol_fee_contract: String,
    /// Fees that are being applied during reinvest of staking rewards
    pub protocol_reward_fee: Decimal, // "1 is 100%, 0.05 is 5%"
    /// Strategy how delegations should be handled
    pub delegation_strategy: Option<DelegationStrategy>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Implements the Cw20 receiver interface
    Receive(Cw20ReceiveMsg),
    /// Bond specified amount of Luna
    Bond {
        receiver: Option<String>,
    },
    /// Donates specified amount of Luna to pool
    Donate {},
    /// Withdraw Luna that have finished unbonding in previous batches
    WithdrawUnbonded {
        receiver: Option<String>,
    },
    /// Add a validator to the whitelist; callable by the owner
    AddValidator {
        validator: String,
    },
    /// Remove a validator from the whitelist; callable by the owner
    RemoveValidator {
        validator: String,
    },
    /// Transfer ownership to another account; will not take effect unless the new owner accepts
    TransferOwnership {
        new_owner: String,
    },
    /// Accept an ownership transfer
    AcceptOwnership {},
    /// Claim staking rewards, swap all for Luna, and restake
    Harvest {},

    TuneDelegations {},
    /// Use redelegations to balance the amounts of Luna delegated to validators
    Rebalance {
        min_redelegation: Option<Uint128>,
    },
    /// Update Luna amounts in unbonding batches to reflect any slashing or rounding errors
    Reconcile {},
    /// Submit the current pending batch of unbonding requests to be unbonded
    SubmitBatch {},
    /// Callbacks; can only be invoked by the contract itself
    Callback(CallbackMsg),

    /// Updates the fee config,
    UpdateConfig {
        /// Contract address where fees are sent
        protocol_fee_contract: Option<String>,
        /// Fees that are being applied during reinvest of staking rewards
        protocol_reward_fee: Option<Decimal>, // "1 is 100%, 0.05 is 5%"

        /// Strategy how delegations should be handled
        delegation_strategy: Option<DelegationStrategy>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveMsg {
    /// Submit an unbonding request to the current unbonding queue; automatically invokes `unbond`
    /// if `epoch_time` has elapsed since when the last unbonding queue was executed.
    QueueUnbond {
        receiver: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CallbackMsg {
    /// Swap Terra stablecoins held by the contract to Luna
    // Swap {},
    /// Following the swaps, stake the Luna acquired to the whitelisted validators
    Reinvest {},

    CheckReceivedCoin {
        snapshot: Coin,
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// The contract's configurations. Response: `ConfigResponse`
    Config {},
    /// The contract's current state. Response: `StateResponse`
    State {},
    /// The contract's current delegation distribution goal. Response: `WantedDelegationsResponse`
    WantedDelegations {},
    /// The contract's delegation distribution goal based on period. Response: `WantedDelegationsResponse`
    SimulateWantedDelegations {
        /// by default uses the next period to look into the future.
        period: Option<u64>,
    },
    /// The current batch on unbonding requests pending submission. Response: `PendingBatch`
    PendingBatch {},
    /// Query an individual batch that has previously been submitted for unbonding but have not yet
    /// fully withdrawn. Response: `Batch`
    PreviousBatch(u64),
    /// Enumerate all previous batches that have previously been submitted for unbonding but have not
    /// yet fully withdrawn. Response: `Vec<Batch>`
    PreviousBatches {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    /// Enumerate all outstanding unbonding requests in a given batch. Response: `Vec<UnbondRequestsByBatchResponseItem>`
    UnbondRequestsByBatch {
        id: u64,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Enumreate all outstanding unbonding requests from given a user. Response: `Vec<UnbondRequestsByUserResponseItem>`
    UnbondRequestsByUser {
        user: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    /// Enumreate all outstanding unbonding requests from given a user. Response: `Vec<UnbondRequestsByUserResponseItemDetails>`
    UnbondRequestsByUserDetails {
        user: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ConfigResponse {
    /// Account who can call certain privileged functions
    pub owner: String,
    /// Pending ownership transfer, awaiting acceptance by the new owner
    pub new_owner: Option<String>,
    /// Address of the Stake token
    pub stake_token: String,
    /// How often the unbonding queue is to be executed, in seconds
    pub epoch_period: u64,
    /// The staking module's unbonding time, in seconds
    pub unbond_period: u64,
    /// Initial set of validators who will receive the delegations
    pub validators: Vec<String>,

    /// Information about applied fees
    pub fee_config: FeeConfig,

    /// Defines how delegations are spread out
    pub delegation_strategy: DelegationStrategy<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct StateResponse {
    /// Total supply to the Stake token
    pub total_ustake: Uint128,
    /// Total amount of uluna staked (bonded)
    pub total_uluna: Uint128,
    /// The exchange rate between ustake and uluna, in terms of uluna per ustake
    pub exchange_rate: Decimal,
    /// Staking rewards currently held by the contract that are ready to be reinvested
    pub unlocked_coins: Vec<Coin>,
    // Amount of uluna currently unbonding
    pub unbonding: Uint128,
    // Amount of uluna currently available as balance of the contract
    pub available: Uint128,
    // Total amount of uluna within the contract (bonded + unbonding + available)
    pub tvl_uluna: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct WantedDelegationsResponse {
    pub tune_time_period: Option<(u64, u64)>,
    pub delegations: Vec<(String, Uint128)>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct WantedDelegationsShare {
    pub tune_time: u64,
    pub tune_period: u64,
    pub shares: Vec<(String, Decimal)>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct SteakStateResponse {
    /// Total supply to the Stake token
    pub total_ustake: Uint128,
    /// Total amount of uluna staked (bonded)
    pub total_uluna: Uint128,
    /// The exchange rate between ustake and uluna, in terms of uluna per ustake
    pub exchange_rate: Decimal,
    /// Staking rewards currently held by the contract that are ready to be reinvested
    pub unlocked_coins: Vec<Coin>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct StaderStateResponse {
    pub state: StaderState,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct StaderState {
    pub total_staked: Uint128,
    pub exchange_rate: Decimal, // shares to token value. 1 share = (ExchangeRate) tokens.
    pub last_reconciled_batch_id: u64,
    pub current_undelegation_batch_id: u64,
    pub last_undelegation_time: Timestamp,
    pub last_swap_time: Timestamp,
    pub last_reinvest_time: Timestamp,
    pub validators: Vec<Addr>,
    pub reconciled_funds_to_withdraw: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PendingBatch {
    /// ID of this batch
    pub id: u64,
    /// Total amount of `ustake` to be burned in this batch
    pub ustake_to_burn: Uint128,
    /// Estimated time when this batch will be submitted for unbonding
    pub est_unbond_start_time: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct FeeConfig {
    /// Contract address where fees are sent
    pub protocol_fee_contract: Addr,
    /// Fees that are being applied during reinvest of staking rewards
    pub protocol_reward_fee: Decimal, // "1 is 100%, 0.05 is 5%"
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Batch {
    /// ID of this batch
    pub id: u64,
    /// Whether this batch has already been reconciled
    pub reconciled: bool,
    /// Total amount of shares remaining this batch. Each `ustake` burned = 1 share
    pub total_shares: Uint128,
    /// Amount of `uluna` in this batch that have not been claimed
    pub uluna_unclaimed: Uint128,
    /// Estimated time when this batch will finish unbonding
    pub est_unbond_end_time: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct UnbondRequest {
    /// ID of the batch
    pub id: u64,
    /// The user's address
    pub user: Addr,
    /// The user's share in the batch
    pub shares: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct UnbondRequestsByBatchResponseItem {
    /// The user's address
    pub user: String,
    /// The user's share in the batch
    pub shares: Uint128,
}

impl From<UnbondRequest> for UnbondRequestsByBatchResponseItem {
    fn from(s: UnbondRequest) -> Self {
        Self {
            user: s.user.into(),
            shares: s.shares,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct UnbondRequestsByUserResponseItem {
    /// ID of the batch
    pub id: u64,
    /// The user's share in the batch
    pub shares: Uint128,
}

impl From<UnbondRequest> for UnbondRequestsByUserResponseItem {
    fn from(s: UnbondRequest) -> Self {
        Self {
            id: s.id,
            shares: s.shares,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct UnbondRequestsByUserResponseItemDetails {
    /// ID of the batch
    pub id: u64,
    /// The user's share in the batch
    pub shares: Uint128,

    // state of pending, unbonding or completed
    pub state: String,

    // The details of the unbonding batch
    pub batch: Option<Batch>,

    // Is set if the unbonding request is still pending
    pub pending: Option<PendingBatch>,
}

pub type MigrateMsg = Empty;

pub fn get_hub_validators(
    querier: &QuerierWrapper,
    hub_addr: impl Into<String>,
) -> StdResult<Vec<String>> {
    let config: ConfigResponse = querier.query_wasm_smart(hub_addr.into(), &QueryMsg::Config {})?;
    Ok(config.validators)
}
