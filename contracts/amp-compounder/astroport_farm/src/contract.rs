use cosmwasm_std::{
    attr, entry_point, from_binary, to_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo,
    Reply, Response, StdError, StdResult, SubMsg,
};

use crate::{
    bond::{bond, bond_assets, bond_to},
    compound::{compound, stake},
    constants::TOKEN_INSTANTIATE_REPLY,
    error::ContractError,
    execute::register_amp_lp_token,
    ownership::{claim_ownership, drop_ownership_proposal, propose_new_owner},
    queries::{query_config, query_exchange_rates, query_state, query_user_info},
    state::{Config, DepositProfitDelay, CONFIG, OWNERSHIP_PROPOSAL},
};

use cw20::Cw20ReceiveMsg;
use eris::{
    adapters::{compounder::Compounder, generator::Generator},
    constants::WEEK,
    helper::{addr_opt_validate, unwrap_reply},
};

use crate::bond::unbond;
use eris::astroport_farm::{
    CallbackMsg, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};

/// ## Description
/// Validates that decimal value is in the range 0 to 1
fn validate_percentage(value: Decimal, field: &str) -> StdResult<()> {
    if value > Decimal::one() {
        Err(StdError::generic_err(field.to_string() + " must be 0 to 1"))
    } else {
        Ok(())
    }
}

fn validate_deposit_profit_delay(deposit_profit_delay_s: u64) -> Result<u64, ContractError> {
    if deposit_profit_delay_s > WEEK {
        Err(ContractError::ConfigValueTooHigh("deposit_profit_delay_s".to_string()))
    } else {
        Ok(deposit_profit_delay_s)
    }
}

/// ## Description
/// Creates a new contract with the specified parameters in the [`InstantiateMsg`].
/// Returns the [`Response`] with the specified attributes if the operation was successful, or a [`ContractError`] if the contract was not created.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    validate_percentage(msg.fee, "fee")?;

    CONFIG.save(
        deps.storage,
        &Config {
            owner: deps.api.addr_validate(&msg.owner)?,
            staking_contract: Generator(deps.api.addr_validate(&msg.staking_contract)?),
            compound_proxy: Compounder(deps.api.addr_validate(&msg.compound_proxy)?),
            controller: deps.api.addr_validate(&msg.controller)?,
            fee: msg.fee,
            fee_collector: deps.api.addr_validate(&msg.fee_collector)?,
            lp_token: deps.api.addr_validate(&msg.liquidity_token)?,
            base_reward_token: deps.api.addr_validate(&msg.base_reward_token)?,
            deposit_profit_delay: DepositProfitDelay {
                seconds: validate_deposit_profit_delay(msg.deposit_profit_delay_s)?,
            },
        },
    )?;

    Ok(Response::new().add_submessage(SubMsg::reply_on_success(
        msg.amp_lp.instantiate(msg.owner, env.contract.address)?,
        TOKEN_INSTANTIATE_REPLY,
    )))
}

/// ## Description
/// Exposes execute functions available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::UpdateConfig {
            compound_proxy,
            controller,
            fee,
            fee_collector,
            deposit_profit_delay_s,
        } => update_config(
            deps,
            info,
            compound_proxy,
            controller,
            fee,
            fee_collector,
            deposit_profit_delay_s,
        ),
        ExecuteMsg::BondAssets {
            assets,
            minimum_receive,
            no_swap,
            slippage_tolerance,
            receiver,
        } => {
            let receiver_addr = addr_opt_validate(deps.api, &receiver)?;
            let receiver_addr = receiver_addr.unwrap_or_else(|| info.sender.clone());
            bond_assets(
                deps,
                env,
                info,
                assets,
                minimum_receive,
                no_swap,
                slippage_tolerance,
                receiver_addr,
            )
        },
        ExecuteMsg::Compound {
            minimum_receive,
            slippage_tolerance,
        } => compound(deps, env, info, minimum_receive, slippage_tolerance),
        ExecuteMsg::ProposeNewOwner {
            owner,
            expires_in,
        } => {
            let config: Config = CONFIG.load(deps.storage)?;

            propose_new_owner(deps, info, env, owner, expires_in, config.owner, OWNERSHIP_PROPOSAL)
                .map_err(|e| e.into())
        },
        ExecuteMsg::DropOwnershipProposal {} => {
            let config: Config = CONFIG.load(deps.storage)?;

            drop_ownership_proposal(deps, info, config.owner, OWNERSHIP_PROPOSAL)
                .map_err(|e| e.into())
        },
        ExecuteMsg::ClaimOwnership {} => {
            claim_ownership(deps, info, env, OWNERSHIP_PROPOSAL, |deps, new_owner| {
                CONFIG.update::<_, StdError>(deps.storage, |mut v| {
                    v.owner = new_owner;
                    Ok(v)
                })?;

                Ok(())
            })
            .map_err(|e| e.into())
        },
        ExecuteMsg::Callback(msg) => handle_callback(deps, env, info, msg),
    }
}

/// ## Description
/// Receives a message of type [`Cw20ReceiveMsg`] and processes it depending on the received template.
/// If the template is not found in the received message, then a [`ContractError`] is returned,
/// otherwise returns a [`Response`] with the specified attributes if the operation was successful
fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::Bond {
            staker_addr,
        }) => bond(deps, env, info, staker_addr.unwrap_or(cw20_msg.sender), cw20_msg.amount),
        Ok(Cw20HookMsg::Unbond {
            receiver,
        }) => unbond(deps, env, info, receiver.unwrap_or(cw20_msg.sender), cw20_msg.amount),
        Err(_) => Err(ContractError::InvalidMessage {}),
    }
}

/// ## Description
/// Updates contract config. Returns a [`ContractError`] on failure or the [`CONFIG`] data will be updated.
#[allow(clippy::too_many_arguments)]
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    compound_proxy: Option<String>,
    controller: Option<String>,
    fee: Option<Decimal>,
    fee_collector: Option<String>,
    deposit_profit_delay_s: Option<u64>,
) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(compound_proxy) = compound_proxy {
        config.compound_proxy = Compounder(deps.api.addr_validate(&compound_proxy)?);
    }

    if let Some(controller) = controller {
        config.controller = deps.api.addr_validate(&controller)?;
    }

    if let Some(fee) = fee {
        validate_percentage(fee, "fee")?;
        config.fee = fee;
    }

    if let Some(fee_collector) = fee_collector {
        config.fee_collector = deps.api.addr_validate(&fee_collector)?;
    }

    if let Some(deposit_profit_delay_s) = deposit_profit_delay_s {
        config.deposit_profit_delay.seconds =
            validate_deposit_profit_delay(deposit_profit_delay_s)?;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![attr("action", "ampf/update_config")]))
}

/// # Description
/// Handle the callbacks describes in the [`CallbackMsg`]. Returns an [`ContractError`] on failure, otherwise returns the [`Response`]
pub fn handle_callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: CallbackMsg,
) -> Result<Response, ContractError> {
    // Callback functions can only be called by this contract itself
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized {});
    }
    match msg {
        CallbackMsg::Stake {
            prev_balance,
            minimum_receive,
        } => stake(deps, env, info, prev_balance, minimum_receive),
        CallbackMsg::BondTo {
            to,
            prev_balance,
            minimum_receive,
        } => bond_to(deps, env, info, to, prev_balance, minimum_receive),
    }
}

#[entry_point]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> StdResult<Response> {
    match reply.id {
        TOKEN_INSTANTIATE_REPLY => register_amp_lp_token(deps, unwrap_reply(reply)?),
        id => Err(StdError::generic_err(format!("invalid reply id: {}; must be 1", id))),
    }
}

/// ## Description
/// Exposes all the queries available in the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::UserInfo {
            addr,
        } => to_binary(&query_user_info(deps, env, addr)?),
        QueryMsg::State {
            addr,
        } => to_binary(&query_state(deps, env, addr)?),
        QueryMsg::ExchangeRates {
            start_after,
            limit,
        } => to_binary(&query_exchange_rates(deps, env, start_after, limit)?),
    }
}
/// ## Description
/// Used for contract migration. Returns a default object of type [`Response`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
