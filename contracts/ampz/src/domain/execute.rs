use std::vec;

use astroport::asset::{native_asset_info, token_asset_info, Asset, AssetInfo, AssetInfoExt};
use cosmwasm_std::{CosmosMsg, DepsMut, Env, MessageInfo, OverflowError, Response, StdResult};
use eris::adapters::whitewhale::WhiteWhale;
use eris::ampz::{DepositMarket, Execution, RepayMarket, Source};

use crate::error::ContractError;
use crate::error::ContractResult;
use crate::extensions::destinationstateext::DestinationStateExt;
use crate::helpers::query_all_delegations;
use crate::protos::authz::MsgExec;
use crate::protos::msgex::CosmosMsgEx;
use crate::protos::proto::MsgWithdrawDelegatorReward;
use crate::state::State;
use eris::constants::CONTRACT_DENOM;
use eris::{
    adapters::asset::AssetInfosEx,
    ampz::{CallbackMsg, DestinationState},
};

pub fn execute_id(deps: DepsMut, env: Env, info: MessageInfo, id: u128) -> ContractResult {
    let state = State::default();

    let execution = state.get_by_id(deps.storage, id)?;

    let user = deps.api.addr_validate(&execution.user)?;

    let last_execution = state.last_execution.load(deps.storage, id)?;
    let next_execution =
        last_execution.checked_add(execution.schedule.interval_s).ok_or_else(|| {
            ContractError::Overflow(OverflowError {
                operation: cosmwasm_std::OverflowOperation::Add,
                operand1: "last_execution".into(),
                operand2: "interval_s".into(),
            })
        })?;

    // it is ok to ignore the schedule e.g. for manual executions.
    let ignore_schedule = info.sender == execution.user;

    if !ignore_schedule && next_execution > env.block.time.seconds() {
        return Err(ContractError::ExecutionInFuture(next_execution));
    }

    if state.is_executing.load(deps.storage).is_ok() {
        return Err(ContractError::IsExecuting {});
    }

    // relevant asset infos that should be used
    let mut asset_infos: Vec<AssetInfo>;
    // user balance start is a snapshot of relevant assets when execution starts (before claiming source yield)
    let user_balance_start: Vec<Asset>;

    // the sub messages are always:
    // 1. claim yield source with ampz (in user wallet)
    // 2. deposit received yield into ampz contract with ampz (in user wallet)
    // --- Rest is executed in the contract
    // 3. Optionally swap to required destination asset (e.g. Amplifier requires utoken deposit)
    // 4. Finish execution by depositing into the destination and sending the result to the user. This also pays operator + protocol fees.
    let mut msgs: Vec<CosmosMsg> = vec![];
    let mut deposit_max_amount: Option<Vec<Asset>> = None;

    match &execution.source {
        Source::Claim => {
            let delegations = query_all_delegations(&deps.querier, &user)?;

            if delegations.is_empty() {
                return Err(ContractError::NoActiveDelegation {});
            }

            asset_infos = vec![native_asset_info(CONTRACT_DENOM.to_string())];
            user_balance_start = asset_infos.query_balances(&deps.querier, &user)?;

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
        Source::AstroRewards {
            lps,
        } => {
            let astroport = state.astroport.load(deps.storage)?;
            asset_infos = astroport.coins;
            // currently all supported tokens will be queried.
            // This could be optimized by storing possible reward tokens for each LP and only query these
            user_balance_start = asset_infos.query_balances(&deps.querier, &user)?;

            let msg =
                astroport.generator.claim_rewards_msg(lps.clone())?.to_authz_msg(&user, &env)?;
            msgs.push(msg);
        },
        Source::Wallet {
            over,
            max_amount,
        } => {
            let current = over.info.query_pool(&deps.querier, &user)?;
            if current <= over.amount {
                return Err(ContractError::BalanceLessThanThreshold {});
            }

            asset_infos = vec![over.info.clone()];
            deposit_max_amount =
                max_amount.map(|max_amount| vec![over.info.with_balance(max_amount)]);

            // instead of querying the user balance, we take the over / min threshold
            user_balance_start = vec![over.clone()];
        },
        Source::ClaimContract {
            claim_type,
        } => {
            match claim_type {
                eris::ampz::ClaimType::WhiteWhaleRewards => {
                    let whitewhale = state.whitewhale.load(deps.storage)?;

                    asset_infos = whitewhale.coins;
                    // currently all supported tokens will be queried.
                    // This could be optimized by storing possible reward tokens for each LP and only query these
                    user_balance_start = asset_infos.query_balances(&deps.querier, &user)?;

                    let msg = WhiteWhale(whitewhale.fee_distributor)
                        .claim_msg()?
                        .to_authz_msg(&user, &env)?;
                    msgs.push(msg);
                },
            }
        },
        Source::WhiteWhaleRewards {
            lps,
        } => {
            let whitewhale = state.whitewhale.load(deps.storage)?;
            asset_infos = whitewhale.coins;
            // currently all supported tokens will be queried.
            // This could be optimized by storing possible reward tokens for each LP and only query these
            user_balance_start = asset_infos.query_balances(&deps.querier, &user)?;

            let mut claim_msgs = lps
                .iter()
                .map(|lp| {
                    WhiteWhale(deps.api.addr_validate(lp)?).claim_msg()?.to_authz_msg(&user, &env)
                })
                .collect::<StdResult<Vec<_>>>()?;

            msgs.append(&mut claim_msgs);
        },
    }

    state.last_execution.save(deps.storage, id, &env.block.time.seconds())?;
    state.is_executing.save(deps.storage, &true)?;

    msgs.push(
        CallbackMsg::AuthzDeposit {
            user_balance_start,
            max_amount: deposit_max_amount,
        }
        .into_cosmos_msg(&env.contract.address, id, &user)?,
    );

    let requires_swap_to = get_swap_asset(&execution, &state, &deps)?;
    if let Some(swap_to) = requires_swap_to {
        if asset_infos.len() == 1 && asset_infos[0] == swap_to {
            // skip swap if it is the same from and to asset
        } else {
            let swap_msg = CallbackMsg::Swap {
                asset_infos: asset_infos.clone(),
                into: swap_to.clone(),
            }
            .into_cosmos_msg(&env.contract.address, id, &user)?;

            // if we swap the results will always be in the native asset (e.g. utoken)
            asset_infos = vec![swap_to];
            msgs.push(swap_msg);
        }
    }

    msgs.push(
        CallbackMsg::FinishExecution {
            destination: execution.destination.to_runtime(asset_infos),
            executor: info.sender,
        }
        .into_cosmos_msg(&env.contract.address, id, &user)?,
    );

    let next_execution = env.block.time.seconds() + execution.schedule.interval_s;
    Ok(Response::new()
        .add_attribute("action", "ampz/execute_id")
        .add_attribute("id", id.to_string())
        .add_attribute("next_execution", next_execution.to_string())
        .add_messages(msgs))
}

fn get_swap_asset(
    execution: &Execution,
    state: &State,
    deps: &DepsMut,
) -> Result<Option<AssetInfo>, ContractError> {
    Ok(match &execution.destination {
        // if we deposit into the amplifier we need to swap to luna
        DestinationState::DepositAmplifier {
            ..
        } => {
            match &execution.source {
                Source::Claim => None,
                Source::Wallet {
                    over,
                    ..
                } => {
                    if over.info != native_asset_info(CONTRACT_DENOM.to_string()) {
                        // if we deposit into amplifier and the deposit asset is not the native chain token, convert it.
                        Some(native_asset_info(CONTRACT_DENOM.to_string()))
                    } else {
                        None
                    }
                },
                _ => Some(native_asset_info(CONTRACT_DENOM.to_string())),
            }
        },
        DestinationState::DepositArbVault {
            ..
        } => Some(native_asset_info(CONTRACT_DENOM.to_string())),
        DestinationState::DepositFarm {
            ..
        } => None,
        DestinationState::DepositLiquidity {
            ..
        } => None,
        DestinationState::SwapTo {
            asset_info,
            ..
        } => Some(asset_info.clone()),
        DestinationState::Repay {
            market,
        } => match market {
            RepayMarket::Capapult => {
                let capa = state.capapult.load(deps.storage)?;
                Some(token_asset_info(capa.stable_cw))
            },
        },
        DestinationState::DepositCollateral {
            market,
        } => match market {
            DepositMarket::Capapult {
                asset_info,
            } => Some(asset_info.clone()),
        },
    })
}
