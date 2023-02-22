use crate::{domain::ownership::OwnershipProposal, error::ContractError};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map};
use eris::arb_vault::{ValidatedConfig, ValidatedFeeConfig};

#[cw_serde]
pub struct BalanceCheckpoint {
    pub vault_available: Uint128,
    pub tvl_utoken: Uint128,
}

#[cw_serde]
pub struct BalanceLocked {
    pub balance: Uint128,
}

#[cw_serde]
pub struct UnbondHistory {
    pub start_time: u64,
    pub release_time: u64,
    pub amount_asset: Uint128,
}

impl UnbondHistory {
    pub fn pool_fee_factor(&self, current_time: u64) -> Decimal {
        // start = 100
        // end = 200
        // current = 130
        // Decimal::from_ratio(130-100,200-100) -> 30 / 100
        let progress = Decimal::from_ratio(
            current_time - self.start_time,
            self.release_time - self.start_time,
        )
        .min(Decimal::one());

        Decimal::one() - progress
    }
}

pub(crate) struct State<'a> {
    pub config: Item<'a, ValidatedConfig>,
    pub fee_config: Item<'a, ValidatedFeeConfig>,
    pub owner: Item<'a, Addr>,
    pub ownership: Item<'a, OwnershipProposal>,
    pub exchange_history: Map<'a, u64, Decimal>,
    pub unbond_history: Map<'a, (Addr, u64), UnbondHistory>,
    pub unbond_id: Item<'a, u64>,
    pub balance_checkpoint: Item<'a, BalanceCheckpoint>,
    pub balance_locked: Item<'a, BalanceLocked>,
}

impl Default for State<'static> {
    fn default() -> Self {
        Self {
            config: Item::new("config"),
            fee_config: Item::new("fee_config"),
            owner: Item::new("owner"),
            ownership: Item::new("ownership"),
            exchange_history: Map::new("exchange_history"),
            unbond_history: Map::new("unbond_history"),
            unbond_id: Item::new("unbond_id"),
            balance_checkpoint: Item::new("balance_checkpoint"),
            balance_locked: Item::new("balance_locked"),
        }
    }
}

impl<'a> State<'a> {
    pub fn assert_owner(
        &self,
        storage: &dyn Storage,
        sender: &Addr,
    ) -> Result<Addr, ContractError> {
        let owner = self.owner.load(storage)?;
        if *sender == owner {
            Ok(owner)
        } else {
            Err(ContractError::Unauthorized {})
        }
    }

    pub fn assert_not_nested(&self, storage: &dyn Storage) -> Result<(), ContractError> {
        let check = self.balance_checkpoint.may_load(storage)?;

        if let Some(..) = check {
            Err(ContractError::AlreadyExecuting {})
        } else {
            Ok(())
        }
    }

    pub fn assert_is_nested(
        &self,
        storage: &dyn Storage,
    ) -> Result<BalanceCheckpoint, ContractError> {
        let check = self.balance_checkpoint.may_load(storage)?;

        if let Some(check) = check {
            Ok(check)
        } else {
            Err(ContractError::NotExecuting {})
        }
    }

    pub fn add_to_unbond_history(
        &self,
        store: &mut dyn Storage,
        sender_addr: Addr,
        element: UnbondHistory,
    ) -> Result<(), ContractError> {
        self.balance_locked.update(store, |mut existing| -> StdResult<_> {
            existing.balance += element.amount_asset;
            Ok(existing)
        })?;

        let id = self.unbond_id.load(store)?;
        self.unbond_history.save(store, (sender_addr, id), &element)?;
        self.unbond_id.save(store, &(id + 1))?;

        Ok(())
    }
}
