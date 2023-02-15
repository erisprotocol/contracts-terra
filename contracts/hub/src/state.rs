use cosmwasm_std::{Addr, Coin, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};

use eris::hub::{
    Batch, DelegationStrategy, FeeConfig, PendingBatch, UnbondRequest, WantedDelegationsShare,
};
use itertools::Itertools;

use crate::{error::ContractError, types::BooleanKey};

pub(crate) struct State<'a> {
    /// Account who can call certain privileged functions
    pub owner: Item<'a, Addr>,
    /// Pending ownership transfer, awaiting acceptance by the new owner
    pub new_owner: Item<'a, Addr>,
    /// Address of the Liquid Staking token
    pub stake_token: Item<'a, Addr>,
    /// How often the unbonding queue is to be executed
    pub epoch_period: Item<'a, u64>,
    /// The staking module's unbonding time, in seconds
    pub unbond_period: Item<'a, u64>,
    /// Validators who will receive the delegations
    pub validators: Item<'a, Vec<String>>,
    /// Coins that can be reinvested
    pub unlocked_coins: Item<'a, Vec<Coin>>,
    /// The current batch of unbonding requests queded to be executed
    pub pending_batch: Item<'a, PendingBatch>,
    /// Previous batches that have started unbonding but not yet finished
    pub previous_batches: IndexedMap<'a, u64, Batch, PreviousBatchesIndexes<'a>>,
    /// Users' shares in unbonding batches
    pub unbond_requests: IndexedMap<'a, (u64, &'a Addr), UnbondRequest, UnbondRequestsIndexes<'a>>,
    /// Fee Config
    pub fee_config: Item<'a, FeeConfig>,
    /// Delegation Strategy
    pub delegation_strategy: Item<'a, DelegationStrategy<Addr>>,
    /// Delegation Distribution
    pub delegation_goal: Item<'a, WantedDelegationsShare>,
    /// Operator who is allowed to vote on props
    pub vote_operator: Item<'a, Addr>,
    /// Specifies wether the contract allows donations
    pub allow_donations: Item<'a, bool>,
}

impl Default for State<'static> {
    fn default() -> Self {
        let pb_indexes = PreviousBatchesIndexes {
            reconciled: MultiIndex::new(
                |d: &Batch| d.reconciled.into(),
                "previous_batches",
                "previous_batches__reconciled",
            ),
        };
        let ubr_indexes = UnbondRequestsIndexes {
            user: MultiIndex::new(
                |d: &UnbondRequest| d.user.clone().into(),
                "unbond_requests",
                "unbond_requests__user",
            ),
        };
        Self {
            owner: Item::new("owner"),
            new_owner: Item::new("new_owner"),
            stake_token: Item::new("stake_token"),
            epoch_period: Item::new("epoch_period"),
            unbond_period: Item::new("unbond_period"),
            validators: Item::new("validators"),
            unlocked_coins: Item::new("unlocked_coins"),
            pending_batch: Item::new("pending_batch"),
            previous_batches: IndexedMap::new("previous_batches", pb_indexes),
            unbond_requests: IndexedMap::new("unbond_requests", ubr_indexes),
            fee_config: Item::new("fee_config"),
            delegation_strategy: Item::new("delegation_strategy"),
            delegation_goal: Item::new("delegation_goal"),
            vote_operator: Item::new("vote_operator"),
            allow_donations: Item::new("allow_donations"),
        }
    }
}

impl<'a> State<'a> {
    pub fn assert_owner(&self, storage: &dyn Storage, sender: &Addr) -> Result<(), ContractError> {
        let owner = self.owner.load(storage)?;
        if *sender == owner {
            Ok(())
        } else {
            Err(ContractError::Unauthorized {})
        }
    }

    pub fn assert_vote_operator(
        &self,
        storage: &dyn Storage,
        sender: &Addr,
    ) -> Result<(), ContractError> {
        let vote_operator =
            self.vote_operator.load(storage).map_err(|_| ContractError::NoVoteOperatorSet {})?;

        if *sender == vote_operator {
            Ok(())
        } else {
            Err(ContractError::UnauthorizedSenderNotVoteOperator {})
        }
    }

    /// active validators returns the list of delegation goal, or if not available (uniform mode) uses the validators list.
    pub fn _active_validators(&self, storage: &dyn Storage) -> Vec<String> {
        self.delegation_goal
            .load(storage)
            .map(|a| a.shares.into_iter().map(|s| s.0).collect_vec())
            .unwrap_or_else(|_| self.validators.load(storage).unwrap_or_default())
    }
}

pub(crate) struct PreviousBatchesIndexes<'a> {
    // pk goes to second tuple element
    pub reconciled: MultiIndex<'a, BooleanKey, Batch, Vec<u8>>,
}

impl<'a> IndexList<Batch> for PreviousBatchesIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Batch>> + '_> {
        let v: Vec<&dyn Index<Batch>> = vec![&self.reconciled];
        Box::new(v.into_iter())
    }
}

pub(crate) struct UnbondRequestsIndexes<'a> {
    // pk goes to second tuple element
    pub user: MultiIndex<'a, String, UnbondRequest, (u64, &'a Addr)>,
}

impl<'a> IndexList<UnbondRequest> for UnbondRequestsIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<UnbondRequest>> + '_> {
        let v: Vec<&dyn Index<UnbondRequest>> = vec![&self.user];

        Box::new(v.into_iter())
    }
}
