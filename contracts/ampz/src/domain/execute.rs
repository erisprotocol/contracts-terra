use std::vec;

use astroport::asset::{native_asset_info, Asset, AssetInfo, AssetInfoExt};
use cosmwasm_std::{CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError, StdResult};

use crate::constants::CONTRACT_DENOM;
use crate::helpers::query_all_delegations;
use crate::protos::authz::MsgExec;
use crate::protos::msgex::CosmosMsgEx;
use crate::protos::proto::MsgWithdrawDelegatorReward;
use crate::state::State;
use eris::{
    adapters::asset::AssetInfosEx,
    ampz::{CallbackMsg, Destination},
};

pub fn execute_id(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: u128,
    user: Option<String>,
) -> StdResult<Response> {
    let state = State::default();

    // state.assert_controller_owner(deps.storage, &info.sender)?;

    let execution = state.get_by_id(deps.storage, id)?;

    if let Some(user) = user {
        if user != execution.user {
            return Err(StdError::generic_err(format!(
                "user does not match. user: {0} execution user: {1}",
                user, execution.user
            )));
        }
    }

    let user = deps.api.addr_validate(&execution.user)?;

    let last_execution = state.last_execution.load(deps.storage, id)?;
    let next_execution = last_execution
        .checked_add(execution.schedule.interval_s)
        .ok_or_else(|| StdError::generic_err("cannot add interval_s"))?;

    // it is ok to ignore the schedule e.g. for manual executions.
    let ignore_schedule = info.sender == execution.user;

    if !ignore_schedule && next_execution > env.block.time.seconds() {
        return Err(StdError::generic_err(format!(
            "next execution in the future: {}",
            next_execution
        )));
    }

    if state.is_executing.load(deps.storage).is_ok() {
        return Err(StdError::generic_err("is already executing"));
    }

    state.last_execution.save(deps.storage, id, &env.block.time.seconds())?;

    let mut msgs: Vec<CosmosMsg> = vec![];
    let asset_infos: Vec<AssetInfo>;
    let user_balance_start: Vec<Asset>;
    let mut deposit_max_amount: Option<Vec<Asset>> = None;

    let mut requires_swap = false;

    match execution.source {
        eris::ampz::Source::Claim => {
            asset_infos = vec![native_asset_info(CONTRACT_DENOM.to_string())];
            user_balance_start = asset_infos.query_balances(&deps.querier, &user)?;
            let delegations = query_all_delegations(&deps.querier, &user)?;

            let mut exec = MsgExec::new();
            exec.grantee = env.contract.address.clone().into();
            exec.msgs = vec![];

            for delegation in delegations {
                let mut msg = MsgWithdrawDelegatorReward::new();
                msg.delegator_address = user.to_string();
                msg.validator_address = delegation.validator.clone();
                exec.msgs.push(msg.to_any()?)
            }

            msgs.push(exec.to_authz_cosmos_msg());
        },
        eris::ampz::Source::AstroRewards {
            lps,
        } => {
            let astroport = state.astroport.load(deps.storage)?;
            asset_infos = astroport.coins;
            user_balance_start = asset_infos.query_balances(&deps.querier, &user)?;

            let msg = astroport.generator.claim_rewards_msg(lps)?.to_authz_msg(&user, &env)?;
            msgs.push(msg);

            if let Destination::DepositAmplifier {} = execution.destination {
                requires_swap = true;
            }
        },
        eris::ampz::Source::Wallet {
            over,
            max_amount,
        } => {
            let current = over.info.query_pool(&deps.querier, &user)?;
            if current <= over.amount {
                return Err(StdError::generic_err(format!(
                    "current balance less than execution start {}",
                    over.amount
                )));
            }

            if let Destination::DepositAmplifier {} = execution.destination {
                if over.info != native_asset_info(CONTRACT_DENOM.to_string()) {
                    requires_swap = true;
                }
            }

            asset_infos = vec![over.info.clone()];
            deposit_max_amount =
                max_amount.map(|max_amount| vec![over.info.with_balance(max_amount)]);
            user_balance_start = vec![over];
        },
    }

    state.is_executing.save(deps.storage, &true)?;

    msgs.push(
        CallbackMsg::AuthzDeposit {
            user_balance_start,
            max_amount: deposit_max_amount,
        }
        .into_cosmos_msg(&env.contract.address, id, &user)?,
    );

    if requires_swap {
        let swap_msg = CallbackMsg::Swap {
            asset_infos: asset_infos.clone(),
            into: native_asset_info(CONTRACT_DENOM.to_string()),
        }
        .into_cosmos_msg(&env.contract.address, id, &user)?;
        msgs.push(swap_msg);
    }

    msgs.push(
        CallbackMsg::FinishExecution {
            destination: execution.destination,
            asset_infos,
            operator: info.sender,
        }
        .into_cosmos_msg(&env.contract.address, id, &user)?,
    );

    Ok(Response::new()
        .add_attribute("action", "ampz/execute_id")
        .add_attribute("id", id.to_string())
        .add_messages(msgs))
}
