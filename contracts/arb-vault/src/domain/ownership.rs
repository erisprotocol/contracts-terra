use cosmwasm_schema::cw_serde;
use cosmwasm_std::{attr, Addr, DepsMut, Env, MessageInfo, Response};

use crate::{
    error::{ContractError, ContractResult},
    state::State,
};

/// ## Description
/// This structure describes the basic settings for creating a request for a change of ownership.
#[cw_serde]
pub struct OwnershipProposal {
    /// a new ownership.
    pub owner: Addr,
    /// time to live a request
    pub ttl: u64,
}

/// ## Description
/// Creates a new request to change ownership. Returns an [`Err`] on failure or returns the [`Response`] with the specified attributes if the operation was successful.
/// ## Executor
/// Only owner can execute it
/// ## Params
/// `deps` is the object of type [`DepsMut`].
///
/// `info` is the object of type [`MessageInfo`].
///
/// `env` is the object of type [`Env`].
///
/// `new_owner` is a new owner.
///
/// `expires_in` is the validity period of the offer to change the owner.
///
/// `owner` is the current owner.
///
/// `proposal` is the object of type [`OwnershipProposal`].
pub fn propose_new_owner(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    new_owner: String,
    expires_in: u64,
) -> ContractResult {
    let state = State::default();
    let owner = state.assert_owner(deps.storage, &info.sender)?;

    let new_owner = deps.api.addr_validate(new_owner.as_str())?;

    // check that owner is not the same
    if new_owner == owner {
        return Err(ContractError::OwnershipProposalOwnerCantBeSame {});
    }

    // max 14 days
    if expires_in > 14 * 24 * 60 * 60 {
        return Err(ContractError::OwnershipProposalExpiryTooLong {});
    }

    state.ownership.save(
        deps.storage,
        &OwnershipProposal {
            owner: new_owner.clone(),
            ttl: env.block.time.seconds() + expires_in,
        },
    )?;

    Ok(Response::new()
        .add_attributes(vec![attr("action", "propose_new_owner"), attr("new_owner", new_owner)]))
}

/// ## Description
/// Removes a request to change ownership. Returns an [`Err`] on failure or returns the [`Response`] with the specified attributes if the operation was successful.
/// ## Executor
/// Only owner can execute it
/// ## Params
/// `deps` is the object of type [`DepsMut`].
///
/// `info` is the object of type [`MessageInfo`].
///
/// `owner` is the current owner.
///
/// `proposal` is the object of type [`OwnershipProposal`].
pub fn drop_ownership_proposal(deps: DepsMut, info: MessageInfo) -> ContractResult {
    let state = State::default();
    state.assert_owner(deps.storage, &info.sender)?;

    state.ownership.remove(deps.storage);

    Ok(Response::new().add_attributes(vec![attr("action", "drop_ownership_proposal")]))
}

/// ## Description
/// Approves owner. Returns an [`Err`] on failure or returns the [`Response`] with the specified attributes if the operation was successful.
/// ## Executor
/// Only owner can execute it
/// ## Params
/// `deps` is the object of type [`DepsMut`].
///
/// `info` is the object of type [`MessageInfo`].
///
/// `env` is the object of type [`Env`].
///
/// `proposal` is the object of type [`OwnershipProposal`].
///
/// `cb` is a type of callback function that takes two parameters of type [`DepsMut`] and [`Addr`].
pub fn claim_ownership(deps: DepsMut, info: MessageInfo, env: Env) -> ContractResult {
    let state = State::default();

    let ownership_proposal: OwnershipProposal = state
        .ownership
        .load(deps.storage)
        .map_err(|_| ContractError::OwnershipProposalNotFound {})?;

    // Check sender
    if info.sender != ownership_proposal.owner {
        return Err(ContractError::Unauthorized {});
    }

    if env.block.time.seconds() > ownership_proposal.ttl {
        return Err(ContractError::OwnershipProposalExpired {});
    }

    state.ownership.remove(deps.storage);

    // run callback
    state.owner.save(deps.storage, &ownership_proposal.owner)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "claim_ownership"),
        attr("new_owner", ownership_proposal.owner),
    ]))
}
