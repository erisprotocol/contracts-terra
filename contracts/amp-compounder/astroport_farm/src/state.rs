use cw_storage_plus::Item;
use eris::adapters::token::Token;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Decimal, Uint128};
use eris::adapters::generator::Generator;
use eris::compound_proxy::Compounder;
use eris::helper::ScalingUint128;

use crate::ownership::OwnershipProposal;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub staking_contract: Generator,
    pub compound_proxy: Compounder,
    pub controller: Addr,
    pub fee: Decimal,
    pub fee_collector: Addr,
    pub lp_token: Addr,
    pub base_reward_token: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub amp_lp_token: Token,
    pub total_bond_share: Uint128,
}

pub const STATE: Item<State> = Item::new("state");

impl State {
    pub fn calc_bond_share(
        &self,
        bond_amount: Uint128,
        lp_balance: Uint128,
        scaling_operation: ScalingOperation,
    ) -> Uint128 {
        if self.total_bond_share.is_zero() || lp_balance.is_zero() {
            bond_amount
        } else {
            match scaling_operation {
                ScalingOperation::Truncate => {
                    bond_amount.multiply_ratio(self.total_bond_share, lp_balance)
                },
                ScalingOperation::Ceil => {
                    bond_amount.multiply_ratio_and_ceil(self.total_bond_share, lp_balance)
                },
            }
        }
    }

    pub fn calc_bond_amount(&self, lp_balance: Uint128, bond_share: Uint128) -> Uint128 {
        if self.total_bond_share.is_zero() {
            Uint128::zero()
        } else {
            lp_balance.multiply_ratio(bond_share, self.total_bond_share)
        }
    }
}

// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// pub struct RewardInfo {
//     pub bond_share: Uint128,
//     pub deposit_amount: Uint128,
//     pub deposit_cost: Uint128,
//     pub deposit_time: u64,
// }

// pub const REWARD: Map<&Addr, RewardInfo> = Map::new("reward");

// const DAY: u64 = 86400;

// impl RewardInfo {
//     pub fn create() -> RewardInfo {
//         RewardInfo {
//             bond_share: Uint128::zero(),
//             deposit_cost: Uint128::zero(),
//             deposit_amount: Uint128::zero(),
//             deposit_time: 0u64,
//         }
//     }

//     pub fn calc_user_balance(&self, state: &State, lp_balance: Uint128, time: u64) -> Uint128 {
//         let amount = state.calc_bond_amount(lp_balance, self.bond_share);
//         let deposit_time = time - self.deposit_time;
//         if deposit_time < DAY && amount > self.deposit_amount {
//             self.deposit_amount + (amount - self.deposit_amount).multiply_ratio(deposit_time, DAY)
//         } else {
//             amount
//         }
//     }
// }

/// Stores the latest proposal to change contract ownership
pub const OWNERSHIP_PROPOSAL: Item<OwnershipProposal> = Item::new("ownership_proposal");

pub enum ScalingOperation {
    Truncate,
    Ceil,
}
