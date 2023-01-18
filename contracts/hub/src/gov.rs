use cosmwasm_std::{
    CosmosMsg, DepsMut, Env, Event, GovMsg, MessageInfo, Response, StdResult, VoteOption,
};

use crate::state::State;

pub fn vote(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    proposal_id: u64,
    vote: VoteOption,
) -> StdResult<Response> {
    let state = State::default();
    state.assert_vote_operator(deps.storage, &info.sender)?;

    let event = Event::new("erishub/voted").add_attribute("prop", proposal_id.to_string());

    let vote = CosmosMsg::Gov(GovMsg::Vote {
        proposal_id,
        vote,
    });

    Ok(Response::new().add_message(vote).add_event(event).add_attribute("action", "erishub/vote"))
}
