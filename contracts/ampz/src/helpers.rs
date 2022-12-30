use cosmwasm_std::{Addr, QuerierWrapper, StdResult};

use crate::{constants::CONTRACT_DENOM, types::Delegation};

pub(crate) fn query_all_delegations(
    querier: &QuerierWrapper,
    delegator_addr: &Addr,
) -> StdResult<Vec<Delegation>> {
    let result: Vec<_> = querier
        .query_all_delegations(delegator_addr)?
        .into_iter()
        .filter(|d| d.amount.denom == CONTRACT_DENOM && !d.amount.amount.is_zero())
        .map(|d| Delegation {
            validator: d.validator,
            amount: d.amount.amount.u128(),
        })
        .collect();

    Ok(result)
}
