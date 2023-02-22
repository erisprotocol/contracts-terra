use cosmwasm_std::{Addr, DepsMut, Event, Response};

use crate::error::{ContractError, ContractResult};
use crate::state::State;

pub fn transfer_ownership(deps: DepsMut, sender: Addr, new_owner: String) -> ContractResult {
    let state = State::default();

    state.assert_owner(deps.storage, &sender)?;
    state.new_owner.save(deps.storage, &deps.api.addr_validate(&new_owner)?)?;

    Ok(Response::new().add_attribute("action", "ampz/transfer_ownership"))
}

pub fn drop_ownership_proposal(deps: DepsMut, sender: Addr) -> ContractResult {
    let state = State::default();

    state.assert_owner(deps.storage, &sender)?;
    state.new_owner.remove(deps.storage);

    Ok(Response::new().add_attribute("action", "ampz/drop_ownership_proposal"))
}

pub fn accept_ownership(deps: DepsMut, sender: Addr) -> ContractResult {
    let state = State::default();

    let previous_owner = state.owner.load(deps.storage)?;
    let new_owner = state.new_owner.load(deps.storage)?;

    if sender != new_owner {
        return Err(ContractError::UnauthorizedSenderNotNewOwner {});
    }

    state.owner.save(deps.storage, &sender)?;
    state.new_owner.remove(deps.storage);

    let event = Event::new("ampz/ownership_transferred")
        .add_attribute("new_owner", new_owner)
        .add_attribute("previous_owner", previous_owner);

    Ok(Response::new().add_event(event).add_attribute("action", "ampz/transfer_ownership"))
}
