use std::{
    cmp,
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

use cosmwasm_std::{Addr, QuerierWrapper, StdResult, Storage, Uint128};

use eris::{
    hub::{Batch, WantedDelegationsShare},
    DecimalCheckedOps,
};

use crate::{
    helpers::query_all_delegations,
    state::State,
    types::{Delegation, Redelegation, Undelegation},
};

type UtokenPerValidator =
    (HashMap<String, Uint128>, Option<u128>, Option<u128>, Option<WantedDelegationsShare>);

//--------------------------------------------------------------------------------------------------
// Minting/burning logics
//--------------------------------------------------------------------------------------------------

/// Compute the amount of Stake token to mint for a specific Luna stake amount. If current total
/// staked amount is zero, we use 1 ustake = 1 uluna; otherwise, we calculate base on the current
/// uluna per ustake ratio.
pub(crate) fn compute_mint_amount(
    ustake_supply: Uint128,
    uluna_to_bond: Uint128,
    current_delegations: &[Delegation],
) -> Uint128 {
    let uluna_bonded: u128 = current_delegations.iter().map(|d| d.amount).sum();
    if uluna_bonded == 0 {
        uluna_to_bond
    } else {
        ustake_supply.multiply_ratio(uluna_to_bond, uluna_bonded)
    }
}

/// Compute the amount of `uluna` to unbond for a specific `ustake` burn amount
///
/// There is no way `ustake` total supply is zero when the user is senting a non-zero amount of `ustake`
/// to burn, so we don't need to handle division-by-zero here
pub(crate) fn compute_unbond_amount(
    ustake_supply: Uint128,
    ustake_to_burn: Uint128,
    current_delegations: &[Delegation],
) -> Uint128 {
    let uluna_bonded: u128 = current_delegations.iter().map(|d| d.amount).sum();
    Uint128::new(uluna_bonded).multiply_ratio(ustake_to_burn, ustake_supply)
}

//--------------------------------------------------------------------------------------------------
// Delegation logics
//--------------------------------------------------------------------------------------------------

/// Given the current delegations made to validators, and a specific amount of `uluna` to unstake,
/// compute the undelegations to make such that the delegated amount to each validator is as even
/// as possible.
///
/// This function is based on Lido's implementation:
/// https://github.com/lidofinance/lido-terra-contracts/blob/v1.0.2/contracts/lido_terra_validators_registry/src/common.rs#L55-102
pub(crate) fn compute_undelegations(
    state: &State,
    storage: &dyn Storage,
    uluna_to_unbond: Uint128,
    current_delegations: &[Delegation],
    validators: Vec<String>,
) -> StdResult<Vec<Undelegation>> {
    let uluna_staked: u128 = current_delegations.iter().map(|d| d.amount).sum();
    let uluna_to_distribute = uluna_staked - uluna_to_unbond.u128();

    let (uluna_per_validator, mut add, mut remove, _) =
        get_uluna_per_validator(state, storage, uluna_to_distribute, &validators, None)?;

    let mut new_undelegations: Vec<Undelegation> = vec![];
    let mut uluna_available = uluna_to_unbond.u128();
    for (_, d) in merge_with_validators(current_delegations, validators).iter().enumerate() {
        let uluna_for_validator =
            get_uluna_for_validator(&uluna_per_validator, d, &mut add, &mut remove);

        let mut uluna_to_undelegate = if d.amount < uluna_for_validator {
            0
        } else {
            d.amount - uluna_for_validator
        };

        uluna_to_undelegate = std::cmp::min(uluna_to_undelegate, uluna_available);
        uluna_available -= uluna_to_undelegate;

        if uluna_to_undelegate > 0 {
            new_undelegations.push(Undelegation::new(&d.validator, uluna_to_undelegate));
        }

        if uluna_available == 0 {
            break;
        }
    }

    Ok(new_undelegations)
}

/// Given a validator who is to be removed from the whitelist, and current delegations made to other
/// validators, compute the new delegations to make such that the delegated amount to each validator
// is as even as possible.
///
/// This function is based on Lido's implementation:
/// https://github.com/lidofinance/lido-terra-contracts/blob/v1.0.2/contracts/lido_terra_validators_registry/src/common.rs#L19-L53
pub(crate) fn compute_redelegations_for_removal(
    state: &State,
    storage: &dyn Storage,
    delegation_to_remove: &Delegation,
    current_delegations: &[Delegation],
    validators: Vec<String>,
) -> StdResult<Vec<Redelegation>> {
    let uluna_staked: u128 = current_delegations.iter().map(|d| d.amount).sum();
    let uluna_to_distribute = uluna_staked + delegation_to_remove.amount;

    let (uluna_per_validator, mut add, mut remove, _) =
        get_uluna_per_validator(state, storage, uluna_to_distribute, &validators, None)?;

    let mut new_redelegations: Vec<Redelegation> = vec![];
    let mut uluna_available = delegation_to_remove.amount;
    for (_, d) in merge_with_validators(current_delegations, validators).iter().enumerate() {
        let uluna_for_validator =
            get_uluna_for_validator(&uluna_per_validator, d, &mut add, &mut remove);

        let mut uluna_to_redelegate = if d.amount > uluna_for_validator {
            0
        } else {
            uluna_for_validator - d.amount
        };

        uluna_to_redelegate = std::cmp::min(uluna_to_redelegate, uluna_available);
        uluna_available -= uluna_to_redelegate;

        if uluna_to_redelegate > 0 {
            new_redelegations.push(Redelegation::new(
                &delegation_to_remove.validator,
                &d.validator,
                uluna_to_redelegate,
            ));
        }

        if uluna_available == 0 {
            break;
        }
    }

    Ok(new_redelegations)
}

fn merge_with_validators(
    current_delegations: &[Delegation],
    validators: Vec<String>,
) -> Vec<Delegation> {
    let hash: HashSet<_> = current_delegations.iter().map(|d| d.validator.to_string()).collect();

    let mut delegations = current_delegations.to_vec();

    for val in validators {
        if !hash.contains(&val) {
            delegations.push(Delegation {
                validator: val,
                amount: 0,
            })
        }
    }

    delegations
}

fn get_uluna_for_validator(
    uluna_per_validator: &HashMap<String, Uint128>,
    delegation: &Delegation,
    add: &mut Option<u128>,
    remove: &mut Option<u128>,
) -> u128 {
    let mut uluna_for_validator =
        uluna_per_validator.get(&delegation.validator).map(|a| a.u128()).unwrap_or_default();
    if let Some(add_set) = *add {
        uluna_for_validator += add_set;
        *add = None;
    }
    if let Some(remove_set) = *remove {
        if uluna_for_validator >= remove_set {
            uluna_for_validator -= remove_set;
            *remove = None;
        }
    }
    uluna_for_validator
}

/// Compute redelegation moves that will make each validator's delegation the targeted amount (hopefully
/// this sentence makes sense)
///
/// This algorithm does not guarantee the minimal number of moves, but is the best I can some up with...
pub(crate) fn compute_redelegations_for_rebalancing(
    state: &State,
    storage: &dyn Storage,
    current_delegations: &[Delegation],
    validators: Vec<String>,
) -> StdResult<Vec<Redelegation>> {
    let uluna_staked: u128 = current_delegations.iter().map(|d| d.amount).sum();

    let (uluna_per_validator, mut add, mut remove, _) =
        get_uluna_per_validator(state, storage, uluna_staked, &validators, None)?;

    // If a validator's current delegated amount is greater than the target amount, Luna will be
    // redelegated _from_ them. They will be put in `src_validators` vector
    // If a validator's current delegated amount is smaller than the target amount, Luna will be
    // redelegated _to_ them. They will be put in `dst_validators` vector
    let mut src_delegations: Vec<Delegation> = vec![];
    let mut dst_delegations: Vec<Delegation> = vec![];
    for (_, d) in merge_with_validators(current_delegations, validators).iter().enumerate() {
        let uluna_for_validator =
            get_uluna_for_validator(&uluna_per_validator, d, &mut add, &mut remove);

        match d.amount.cmp(&uluna_for_validator) {
            Ordering::Greater => {
                src_delegations.push(Delegation::new(&d.validator, d.amount - uluna_for_validator));
            },
            Ordering::Less => {
                dst_delegations.push(Delegation::new(&d.validator, uluna_for_validator - d.amount));
            },
            Ordering::Equal => (),
        }
    }

    let mut new_redelegations: Vec<Redelegation> = vec![];
    while !src_delegations.is_empty() && !dst_delegations.is_empty() {
        let src_delegation = src_delegations[0].clone();
        let dst_delegation = dst_delegations[0].clone();
        let uluna_to_redelegate = cmp::min(src_delegation.amount, dst_delegation.amount);

        if src_delegation.amount == uluna_to_redelegate {
            src_delegations.remove(0);
        } else {
            src_delegations[0].amount -= uluna_to_redelegate;
        }

        if dst_delegation.amount == uluna_to_redelegate {
            dst_delegations.remove(0);
        } else {
            dst_delegations[0].amount -= uluna_to_redelegate;
        }

        new_redelegations.push(Redelegation::new(
            &src_delegation.validator,
            &dst_delegation.validator,
            uluna_to_redelegate,
        ));
    }

    Ok(new_redelegations)
}

/// Load uluna per validator
/// If no goal is provided, the stored goal or uniform distribution is used.
pub(crate) fn get_uluna_per_validator_prepared(
    state: &State,
    storage: &dyn Storage,
    querier: &QuerierWrapper,
    contract: &Addr,
    goal: Option<WantedDelegationsShare>,
) -> StdResult<UtokenPerValidator> {
    let current_delegations = query_all_delegations(querier, contract)?;
    let uluna_staked: u128 = current_delegations.iter().map(|d| d.amount).sum();
    let validators = state.validators.load(storage)?;
    get_uluna_per_validator(state, storage, uluna_staked, &validators, goal)
}

pub(crate) fn get_uluna_per_validator(
    state: &State,
    storage: &dyn Storage,
    uluna_staked: u128,
    validators: &[String],
    goal: Option<WantedDelegationsShare>,
) -> StdResult<UtokenPerValidator> {
    let uluna_staked_uint = Uint128::new(uluna_staked);
    let delegation_goal = if goal.is_some() {
        goal
    } else {
        state.delegation_goal.may_load(storage)?
    };

    let uluna_per_validator: Option<HashMap<_, _>> =
        if let Some(delegation_goal) = delegation_goal.clone() {
            if !delegation_goal.shares.is_empty() {
                // calculate via distribution
                Some(
                    delegation_goal
                        .shares
                        .into_iter()
                        .map(|d| -> StdResult<(String, Uint128)> {
                            Ok((d.0, d.1.checked_mul_uint(uluna_staked_uint)?))
                        })
                        .collect::<StdResult<HashMap<_, _>>>()?,
                )
            } else {
                None
            }
        } else {
            None
        };

    let uluna_per_validator = uluna_per_validator.unwrap_or_else(|| {
        let validator_count = validators.len() as u128;
        let uluna_per_validator = uluna_staked / validator_count;
        validators.iter().map(|d| (d.clone(), Uint128::new(uluna_per_validator))).collect()
    });
    let total: u128 = uluna_per_validator.iter().map(|a| a.1.u128()).sum();
    let add = if total < uluna_staked {
        Some(uluna_staked - total)
    } else {
        None
    };
    let remove = if total > uluna_staked {
        Some(total - uluna_staked)
    } else {
        None
    };
    Ok((uluna_per_validator, add, remove, delegation_goal))
}

//--------------------------------------------------------------------------------------------------
// Batch logics
//--------------------------------------------------------------------------------------------------

/// If the received uluna amount after the unbonding period is less than expected, e.g. due to rounding
/// error or the validator(s) being slashed, then deduct the difference in amount evenly from each
/// unreconciled batch.
///
/// The idea of "reconciling" is based on Stader's implementation:
/// https://github.com/stader-labs/stader-liquid-token/blob/v0.2.1/contracts/staking/src/contract.rs#L968-L1048
pub(crate) fn reconcile_batches(batches: &mut [Batch], uluna_to_deduct: Uint128) {
    let batch_count = batches.len() as u128;
    let uluna_per_batch = uluna_to_deduct.u128() / batch_count;
    let remainder = uluna_to_deduct.u128() % batch_count;

    for (i, batch) in batches.iter_mut().enumerate() {
        let remainder_for_batch: u128 = if (i + 1) as u128 <= remainder {
            1
        } else {
            0
        };
        let uluna_for_batch = uluna_per_batch + remainder_for_batch;

        batch.uluna_unclaimed -= Uint128::new(uluna_for_batch);
        batch.reconciled = true;
    }
}

/// If all funds are available we still need to mark batches as reconciled
pub(crate) fn mark_reconciled_batches(batches: &mut [Batch]) {
    for (_, batch) in batches.iter_mut().enumerate() {
        batch.reconciled = true;
    }
}
