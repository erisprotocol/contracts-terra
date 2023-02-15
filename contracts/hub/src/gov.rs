use cosmwasm_std::{
    CosmosMsg, Decimal, DepsMut, Env, Event, Fraction, GovMsg, MessageInfo, Response,
};
use itertools::Itertools;
use protobuf::SpecialFields;

use crate::{
    error::ContractResult,
    protos::proto::{MsgVoteWeighted, VoteOption, WeightedVoteOption},
    state::State,
};

pub fn vote(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    proposal_id: u64,
    vote: cosmwasm_std::VoteOption,
) -> ContractResult {
    let state = State::default();
    state.assert_vote_operator(deps.storage, &info.sender)?;

    let event = Event::new("erishub/voted").add_attribute("prop", proposal_id.to_string());

    let vote = CosmosMsg::Gov(GovMsg::Vote {
        proposal_id,
        vote,
    });

    Ok(Response::new().add_message(vote).add_event(event).add_attribute("action", "erishub/vote"))
}

pub fn vote_weighted(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    proposal_id: u64,
    votes: Vec<(Decimal, cosmwasm_std::VoteOption)>,
) -> ContractResult {
    let state = State::default();
    state.assert_vote_operator(deps.storage, &info.sender)?;

    let event = Event::new("erishub/voted_weighted").add_attribute("prop", proposal_id.to_string());

    let vote = MsgVoteWeighted {
        proposal_id,
        voter: _env.contract.address.to_string(),
        options: votes
            .into_iter()
            .map(|vote| WeightedVoteOption {
                special_fields: SpecialFields::default(),
                option: match vote.1 {
                    cosmwasm_std::VoteOption::Yes => VoteOption::VOTE_OPTION_YES.into(),
                    cosmwasm_std::VoteOption::No => VoteOption::VOTE_OPTION_NO.into(),
                    cosmwasm_std::VoteOption::Abstain => VoteOption::VOTE_OPTION_ABSTAIN.into(),
                    cosmwasm_std::VoteOption::NoWithVeto => {
                        VoteOption::VOTE_OPTION_NO_WITH_VETO.into()
                    },
                },
                weight: vote.0.numerator().to_string(),
            })
            .collect_vec(),
        special_fields: SpecialFields::default(),
    };

    let vote = vote.to_cosmos_msg();

    Ok(Response::new()
        .add_message(vote)
        .add_event(event)
        .add_attribute("action", "erishub/vote_weighted"))
}
