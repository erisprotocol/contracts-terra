use crate::astro_gov::{AstroGov, AstroGovUnchecked};
use astroport::restricted_vector::RestrictedVector;
use cosmwasm_std::{to_binary, Addr, CosmosMsg, Decimal, StdResult, Uint128, WasmMsg};
use cw20::Cw20ReceiveMsg;
use eris::adapters::generator::Generator;
use eris::helper::ScalingUint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    pub generator: String,
    pub astro_gov: AstroGovUnchecked,
    pub owner: String,
    pub controller: String,
    pub astro_token: String,
    pub fee_collector: String,
    pub max_quota: Uint128,
    pub staker_rate: Decimal,
    pub boost_fee: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    pub generator: Generator,
    pub astro_gov: AstroGov,
    pub owner: Addr,
    pub controller: Addr,
    pub astro_token: Addr,
    pub fee_collector: Addr,
    pub max_quota: Uint128,
    pub staker_rate: Decimal,
    pub boost_fee: Decimal,
}

pub fn zero_address() -> Addr {
    Addr::unchecked("")
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct PoolInfo {
    pub total_bond_share: Uint128,
    pub reward_indexes: RestrictedVector<Addr, Decimal>,
    pub prev_reward_user_index: Decimal,
    pub prev_reward_debt_proxy: RestrictedVector<Addr, Uint128>,
    #[serde(default)]
    pub last_reconcile: u64,
}

impl PoolInfo {
    pub fn calc_bond_share(
        &self,
        total_bond_amount: Uint128,
        amount: Uint128,
        ceiling: bool,
    ) -> Uint128 {
        if self.total_bond_share.is_zero() || total_bond_amount.is_zero() {
            amount
        } else if ceiling {
            amount.multiply_ratio_and_ceil(self.total_bond_share, total_bond_amount)
        } else {
            amount.multiply_ratio(self.total_bond_share, total_bond_amount)
        }
    }

    pub fn calc_bond_amount(&self, total_bond_amount: Uint128, share: Uint128) -> Uint128 {
        if self.total_bond_share.is_zero() {
            Uint128::zero()
        } else {
            total_bond_amount.multiply_ratio(share, self.total_bond_share)
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserInfo {
    pub bond_share: Uint128,
    pub reward_indexes: RestrictedVector<Addr, Decimal>,
    pub pending_rewards: RestrictedVector<Addr, Uint128>,
}

impl UserInfo {
    pub fn create(pool_info: &PoolInfo) -> UserInfo {
        UserInfo {
            bond_share: Uint128::zero(),
            reward_indexes: pool_info.reward_indexes.clone(),
            pending_rewards: RestrictedVector::default(),
        }
    }

    pub fn to_response(
        &self,
        pool_info: &PoolInfo,
        total_bond_amount: Uint128,
    ) -> UserInfoResponse {
        UserInfoResponse {
            bond_share: self.bond_share,
            bond_amount: pool_info.calc_bond_amount(total_bond_amount, self.bond_share),
            reward_indexes: self.reward_indexes.clone(),
            pending_rewards: self.pending_rewards.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserInfoResponse {
    pub bond_share: Uint128,
    pub bond_amount: Uint128,
    pub reward_indexes: RestrictedVector<Addr, Decimal>,
    pub pending_rewards: RestrictedVector<Addr, Uint128>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema, Default)]
pub struct StakingState {
    pub total_bond_share: Uint128,
    pub reward_index: Decimal,
    pub next_claim_period: u64,
    pub total_unstaking_amount: Uint128,
    pub total_unstaked_amount: Uint128,
    pub unstaking_period: u64,
}

impl StakingState {
    pub fn calc_bond_share(
        &self,
        total_bond_amount: Uint128,
        amount: Uint128,
        ceiling: bool,
    ) -> Uint128 {
        let total_bond_amount = total_bond_amount.saturating_sub(self.total_unstaking_amount);
        if self.total_bond_share.is_zero() || total_bond_amount.is_zero() {
            amount
        } else if ceiling {
            amount.multiply_ratio_and_ceil(self.total_bond_share, total_bond_amount)
        } else {
            amount.multiply_ratio(self.total_bond_share, total_bond_amount)
        }
    }

    pub fn calc_bond_amount(&self, total_bond_amount: Uint128, share: Uint128) -> Uint128 {
        let total_bond_amount = total_bond_amount.saturating_sub(self.total_unstaking_amount);
        if self.total_bond_share.is_zero() {
            Uint128::zero()
        } else {
            total_bond_amount.multiply_ratio(share, self.total_bond_share)
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct StakerInfo {
    pub bond_share: Uint128,
    pub reward_index: Decimal,
    pub pending_reward: Uint128,
    pub unstaking_amount: Uint128,
    pub unstaked_amount: Uint128,
    pub unstaking_period: u64,
}

impl StakerInfo {
    pub fn create(state: &StakingState) -> StakerInfo {
        StakerInfo {
            bond_share: Uint128::zero(),
            reward_index: state.reward_index,
            pending_reward: Uint128::zero(),
            unstaking_amount: Uint128::zero(),
            unstaked_amount: Uint128::zero(),
            unstaking_period: state.unstaking_period,
        }
    }

    pub fn update_staking(&mut self, state: &StakingState) {
        if state.unstaking_period > self.unstaking_period {
            self.unstaked_amount += self.unstaking_amount;
            self.unstaking_amount = Uint128::zero();
            self.unstaking_period = state.unstaking_period;
        }
    }

    pub fn to_response(
        &self,
        state: &StakingState,
        total_bond_amount: Uint128,
    ) -> StakerInfoResponse {
        StakerInfoResponse {
            bond_share: self.bond_share,
            bond_amount: state.calc_bond_amount(total_bond_amount, self.bond_share),
            reward_index: self.reward_index,
            pending_reward: self.pending_reward,
            unstaking_amount: self.unstaking_amount,
            unstaked_amount: self.unstaked_amount,
            unstaking_period: self.unstaking_period,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct StakerInfoResponse {
    pub bond_share: Uint128,
    pub bond_amount: Uint128,
    pub reward_index: Decimal,
    pub pending_reward: Uint128,
    pub unstaking_amount: Uint128,
    pub unstaked_amount: Uint128,
    pub unstaking_period: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema, Default)]
pub struct RewardInfo {
    pub reconciled_amount: Uint128,
    pub fee: Uint128,
    pub staker_income: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    Callback(CallbackMsg),

    // config
    UpdateConfig {
        controller: Option<String>,
        boost_fee: Option<Decimal>,
    },

    // controller's actions
    UpdateParameters {
        max_quota: Option<Uint128>,
        staker_rate: Option<Decimal>,
    },
    ControllerVote {
        votes: Vec<(String, u16)>,
    },
    SendIncome {},

    // from generator
    /// Update rewards and return it to user.
    ClaimRewards {
        /// the LP token contract address
        lp_tokens: Vec<String>,
    },
    /// Withdraw LP tokens from the Generator
    Withdraw {
        /// The address of the LP token to withdraw
        lp_token: String,
        /// The amount to withdraw
        amount: Uint128,
    },

    // owner
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

    // stakers
    Relock {},
    RequestUnstake {
        amount: Uint128,
    },
    WithdrawUnstaked {
        amount: Option<Uint128>,
    },
    ClaimIncome {},
}

impl ExecuteMsg {
    pub fn to_cosmos_msg(&self, contract_addr: &Addr) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: String::from(contract_addr),
            msg: to_binary(self)?,
            funds: vec![],
        }))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CallbackMsg {
    AfterBondClaimed {
        lp_token: Addr,
        prev_balances: Vec<(Addr, Uint128)>,
    },
    Deposit {
        lp_token: Addr,
        staker_addr: Addr,
        amount: Uint128,
    },
    Withdraw {
        lp_token: Addr,
        staker_addr: Addr,
        amount: Uint128,
    },
    AfterBondChanged {
        lp_token: Addr,
    },
    ClaimRewards {
        lp_token: Addr,
        staker_addr: Addr,
    },
    AfterStakingClaimed {
        prev_balance: Uint128,
    },
}

impl CallbackMsg {
    pub fn to_cosmos_msg(&self, contract_addr: &Addr) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: String::from(contract_addr),
            msg: to_binary(&ExecuteMsg::Callback(self.clone()))?,
            funds: vec![],
        }))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    // from generator
    Deposit {},

    // ASTRO staking
    Stake {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},

    PoolInfo {
        lp_token: String,
    },
    UserInfo {
        lp_token: String,
        user: String,
    },
    RewardInfo {
        token: String,
    },

    // from generator
    PendingToken {
        lp_token: String,
        user: String,
    },
    Deposit {
        lp_token: String,
        user: String,
    },

    // staker
    StakingState {},
    StakerInfo {
        user: String,
    },
}
