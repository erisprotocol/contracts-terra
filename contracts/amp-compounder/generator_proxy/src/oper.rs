use crate::error::ContractError;
use crate::model::Config;
use crate::state::{CONFIG, REWARD_INFO};
use astroport::asset::token_asset;
use cosmwasm_std::{
    CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128,
};
use eris::adapters::asset::AssetEx;

pub fn validate_percentage(value: Decimal, field: &str) -> StdResult<()> {
    if value > Decimal::one() {
        Err(StdError::generic_err(field.to_string() + " cannot greater than 1"))
    } else {
        Ok(())
    }
}
pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    controller: Option<String>,
    boost_fee: Option<Decimal>,
) -> Result<Response, ContractError> {
    // only owner can update
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(controller) = controller {
        config.controller = deps.api.addr_validate(&controller)?;
    }

    if let Some(boost_fee) = boost_fee {
        validate_percentage(boost_fee, "boost_fee")?;
        config.boost_fee = boost_fee;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

pub fn execute_update_parameters(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    max_quota: Option<Uint128>,
    staker_rate: Option<Decimal>,
) -> Result<Response, ContractError> {
    // only controller can update
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.controller {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(max_quota) = max_quota {
        config.max_quota = max_quota;
    }

    if let Some(staker_rate) = staker_rate {
        validate_percentage(staker_rate, "staker_rate")?;
        config.staker_rate = staker_rate;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

pub fn execute_controller_vote(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    votes: Vec<(String, u16)>,
) -> Result<Response, ContractError> {
    // only controller can vote
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.controller {
        return Err(ContractError::Unauthorized {});
    }

    let vote_msg = config.astro_gov.controller_vote_msg(votes)?;

    Ok(Response::new().add_message(vote_msg))
}

pub fn execute_send_income(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // this method can only invoked by controller
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.controller {
        return Err(ContractError::Unauthorized {});
    }

    let mut messages: Vec<CosmosMsg> = vec![];
    let mut reward_info = REWARD_INFO.load(deps.storage, &config.astro_token)?;
    let fee = reward_info.fee;
    reward_info.fee = Uint128::zero();
    reward_info.reconciled_amount -= fee;

    // save
    REWARD_INFO.save(deps.storage, &config.astro_token, &reward_info)?;

    if !fee.is_zero() {
        messages.push(token_asset(config.astro_token, fee).transfer_msg(&config.fee_collector)?);
    }

    Ok(Response::new().add_messages(messages))
}

pub fn query_config(deps: Deps, _env: Env) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}
