use std::ops::Div;

use cosmwasm_schema::cw_serde;
use cw_storage_plus::{Item, Map};
use eris::adapters::compounder::Compounder;
use eris::adapters::token::Token;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Decimal, Order, StdResult, Storage, Uint128};
use eris::adapters::generator::Generator;

use crate::{constants::RELAVANT_EXCHANGE_RATES, ownership::OwnershipProposal};

#[cw_serde]
pub struct Config {
    /// owner of the contract that can execute administrative functions
    pub owner: Addr,
    // contract where the LP should be staked at
    pub staking_contract: Generator,
    // contract for swapping funds to an LP
    pub compound_proxy: Compounder,
    // address that is allowed to compound rewards
    pub controller: Addr,
    // performance fee
    pub fee: Decimal,
    // performance fee receiver
    pub fee_collector: Addr,
    // based on the tracked exchange rate new deposits will only be profitable after the delay.
    // This should be set higher than the compounding interval, so that rewards can't be gamed.
    #[serde(default)]
    pub deposit_profit_delay: DepositProfitDelay,
    // lp token that is being used
    pub lp_token: Addr,
    // default reward token
    pub base_reward_token: Addr,
}

#[cw_serde]
#[derive(Default)]
pub struct DepositProfitDelay {
    pub seconds: u64,
}

impl DepositProfitDelay {
    pub fn calc_adjusted_share(
        &self,
        storage: &mut dyn Storage,
        bond_share: Uint128,
    ) -> StdResult<Uint128> {
        if self.seconds == 0 {
            return Ok(bond_share);
        }

        let exchange_rates = EXCHANGE_HISTORY
            .range(storage, None, None, Order::Descending)
            .take(RELAVANT_EXCHANGE_RATES)
            .collect::<StdResult<Vec<(u64, Decimal)>>>()?;

        if exchange_rates.len() < 2 {
            Ok(bond_share)
        } else {
            let current = exchange_rates[0];
            let last = exchange_rates[exchange_rates.len() - 1];

            let delta_time_s = current.0 - last.0;
            // if the exchange rate has been reduced (which cant happen), ignore it.
            let delta_rate = current.1.checked_sub(last.1).unwrap_or_default();
            // specifies how much the exchange rate has increased in comparison to the start point. (e.g. 50% since last)
            let delta_rate_percent = delta_rate.div(last.1);

            // delta_rate_percent = delta_rate / start
            // factor = delta_rate_percent / delta_time_s * deposit_profit_delay_s = delta_rate_percent * (deposit_profit_delay_s / delta_time_s)
            // e.g. delta_rate_percent 0.1, delta_time_s: 3d, deposit_porift_delay_s: 1d
            // -> factor = 0.03333
            // adjusted_share = share / (1 + factor)

            let factor_plus_one = delta_rate_percent
                .checked_mul(Decimal::from_ratio(self.seconds, delta_time_s))?
                .checked_add(Decimal::one())?;

            let adjusted_share = bond_share * Decimal::one().div(factor_plus_one);
            println!(
                "delta_time_s {0}, delta_rate_percent {1}, factor_plus_one {2}, adjusted_share {3}",
                delta_time_s, delta_rate_percent, factor_plus_one, adjusted_share
            );
            Ok(adjusted_share)
        }
    }
}

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct State {
    pub amp_lp_token: Token,
    pub total_bond_share: Uint128,
}

pub const STATE: Item<State> = Item::new("state");

impl State {
    pub fn calc_bond_share(&self, bond_amount: Uint128, lp_balance: Uint128) -> Uint128 {
        if self.total_bond_share.is_zero() || lp_balance.is_zero() {
            bond_amount
        } else {
            bond_amount.multiply_ratio(self.total_bond_share, lp_balance)
        }
    }

    pub fn calc_bond_amount(&self, lp_balance: Uint128, bond_share: Uint128) -> Uint128 {
        if self.total_bond_share.is_zero() {
            Uint128::zero()
        } else {
            lp_balance.multiply_ratio(bond_share, self.total_bond_share)
        }
    }

    pub(crate) fn calc_exchange_rate(&self, total_lp: Uint128) -> Decimal {
        if self.total_bond_share.is_zero() {
            Decimal::one()
        } else {
            Decimal::from_ratio(total_lp, self.total_bond_share)
        }
    }
}

pub const EXCHANGE_HISTORY: Map<u64, Decimal> = Map::new("exchange_history");

/// Stores the latest proposal to change contract ownership
pub const OWNERSHIP_PROPOSAL: Item<OwnershipProposal> = Item::new("ownership_proposal");
