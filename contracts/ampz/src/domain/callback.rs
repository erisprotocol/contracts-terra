use std::convert::TryInto;
use std::{cmp, vec};

use astroport::asset::{native_asset_info, token_asset_info, Asset, AssetInfo, AssetInfoExt};
use cosmwasm_std::{
    attr, Addr, Attribute, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
    Uint128,
};
use eris::ampz::{CallbackMsg, CallbackWrapper, DepositMarket, DestinationRuntime, RepayMarket};

use crate::adapters::capapult::{CapapultLocker, CapapultMarket};
use crate::error::{ContractError, ContractResult};
use crate::protos::msgex::{CosmosMsgEx, CosmosMsgsEx};
use crate::state::State;
use eris::adapters::ampz::Ampz;
use eris::adapters::asset::{AssetEx, AssetInfosEx, AssetsEx};
use eris::adapters::farm::Farm;
use eris::constants::CONTRACT_DENOM;
use eris::helper::funds_or_allowance;
use eris::helpers::bps::BasicPoints;
use itertools::Itertools;

pub fn callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    callback_wrapper: CallbackWrapper,
) -> ContractResult {
    if env.contract.address != info.sender {
        return Err(ContractError::CallbackOnlyCalledByContract {});
    }

    let state = State::default();
    let user = callback_wrapper.user;

    let mut msgs: Vec<CosmosMsg> = vec![];
    let mut attrs: Vec<Attribute> = vec![];

    // we are not revalidating the id and the user, as the callback comes from ourself in a trusted way

    match callback_wrapper.message {
        CallbackMsg::AuthzDeposit {
            user_balance_start,
            max_amount,
        } => {
            attrs.push(attr("type", "authz_deposit"));

            // the snapshot of the user balance is in the callback message
            // the contract queries the same assets again and takes a diff of what has been added
            let balances =
                user_balance_start.query_balance_diff(&deps.querier, &user, max_amount)?;

            if balances.is_empty() {
                return Err(ContractError::NothingToDeposit {});
            }

            // rest is used to create allowance or deposit messages into the ampz contract
            let (funds, allowances) =
                funds_or_allowance(&env, &env.contract.address, &balances, None)?;
            for allowance in allowances {
                msgs.push(allowance.to_authz_msg(user.clone(), &env)?);
            }
            for asset in balances.iter() {
                attrs.push(attr("amount", asset.to_string()));
            }
            msgs.push(
                Ampz(env.contract.address.clone())
                    .deposit(balances, funds)?
                    .to_authz_msg(user, &env)?,
            );
        },

        CallbackMsg::Swap {
            asset_infos,
            into,
        } => {
            attrs.push(attr("type", "swap"));

            // this swaps all specified assets to the "into" asset. Ignoring already correctly swapped assets.
            let asset_infos = asset_infos.into_iter().filter(|info| *info != into).collect_vec();
            let balances = asset_infos
                .query_balances(&deps.querier, &env.contract.address)?
                .into_iter()
                .filter(|asset| !asset.amount.is_zero())
                .collect_vec();

            if balances.is_empty() {
                // when executing a swap and nothing needs to be swapped, we can still continue
                attrs.push(attr("skipped-swap", "1"));
                attrs.push(attr("to", into.to_string()));
            } else {
                let zapper = state.zapper.load(deps.storage)?;

                let (funds, mut allowances) = funds_or_allowance(&env, &zapper.0, &balances, None)?;

                for asset in balances.iter() {
                    attrs.push(attr("from", asset.to_string()));
                }

                let multi_swap_msg = zapper.multi_swap_msg(balances, into.clone(), funds, None)?;

                // it uses the ERIS zapper multi-swap feature
                msgs.append(&mut allowances);
                msgs.push(multi_swap_msg);
                attrs.push(attr("to", into.to_string()));
            }
        },

        CallbackMsg::FinishExecution {
            destination,
            executor,
        } => {
            match destination {
                DestinationRuntime::DepositAmplifier {
                    receiver,
                } => {
                    attrs.push(attr("type", "deposit_amplifier"));
                    let main_token = native_asset_info(CONTRACT_DENOM.to_string());
                    let amount = main_token.query_pool(&deps.querier, env.contract.address)?;
                    let balances = vec![main_token.with_balance(amount)];

                    if amount.is_zero() {
                        return Err(ContractError::NothingToDeposit {});
                    }

                    let balances =
                        pay_fees(&state, &deps, &mut msgs, &mut attrs, balances, executor, &user)?;

                    // always 1 result if it inputs a non-zero token
                    let balance = balances.first().unwrap();

                    let receiver: String = receiver.unwrap_or(user).into();
                    let hub = state.hub.load(deps.storage)?;
                    let bond_msg =
                        hub.bond_msg(CONTRACT_DENOM, balance.amount.u128(), Some(receiver))?;
                    msgs.push(bond_msg);
                },

                DestinationRuntime::DepositArbVault {
                    receiver,
                } => {
                    attrs.push(attr("type", "deposit_arb_vault"));
                    let main_token = native_asset_info(CONTRACT_DENOM.to_string());
                    let amount = main_token.query_pool(&deps.querier, env.contract.address)?;
                    let balances = vec![main_token.with_balance(amount)];

                    if amount.is_zero() {
                        return Err(ContractError::NothingToDeposit {});
                    }

                    let balances =
                        pay_fees(&state, &deps, &mut msgs, &mut attrs, balances, executor, &user)?;

                    // always 1 result if it inputs a non-zero token
                    let balance = balances.first().unwrap();

                    let receiver: String = receiver.unwrap_or(user).into();
                    let arb_vault = state.arb_vault.load(deps.storage)?;
                    let deposit_msg = arb_vault.deposit_msg(
                        CONTRACT_DENOM,
                        balance.amount.u128(),
                        Some(receiver),
                    )?;
                    msgs.push(deposit_msg);
                },

                DestinationRuntime::DepositFarm {
                    asset_infos,
                    farm,
                    receiver,
                } => {
                    attrs.push(attr("type", "deposit_farm"));
                    let balances =
                        asset_infos.query_balances(&deps.querier, &env.contract.address)?;
                    let balances =
                        pay_fees(&state, &deps, &mut msgs, &mut attrs, balances, executor, &user)?;

                    let receiver: String = receiver.unwrap_or(user).into();
                    deposit_in_farm(&deps, farm, &env, receiver, balances, &mut msgs)?;
                },
                DestinationRuntime::SendSwapResultToUser {
                    asset_info,
                    receiver,
                } => {
                    // at this point the swap has already been executed and we just need to send the result back to the user + pay fees.
                    attrs.push(attr("type", "swap_to"));

                    let receiver = receiver.unwrap_or_else(|| user.clone());
                    pay_fees_and_send_to_receiver(
                        &deps, &env, &state, &mut msgs, &mut attrs, asset_info, executor, &user,
                        &receiver,
                    )?;
                },

                DestinationRuntime::Repay {
                    market,
                } => {
                    attrs.push(attr("type", "repay"));
                    match market {
                        RepayMarket::Capapult => {
                            attrs.push(attr("market", "capapult"));

                            let capa = state.capapult.load(deps.storage)?;
                            let asset_info = token_asset_info(capa.stable_cw);

                            // send fees and rest of the funds back to the user
                            let asset = pay_fees_and_send_to_receiver(
                                &deps, &env, &state, &mut msgs, &mut attrs, asset_info, executor,
                                &user,
                                // in case of capa, it can only execute the deposit for the user
                                &user,
                            )?;

                            let capapult_market = CapapultMarket(capa.market);

                            // check if user has an open loan
                            let max_repay: Uint128 = capapult_market
                                .query_borrower_info(&deps.querier, user.to_string())
                                .map(|a| a.loan_amount)
                                .unwrap_or_default()
                                .try_into()
                                .unwrap_or_default();

                            if max_repay.is_zero() {
                                return Err(ContractError::NothingToDeposit {});
                            }

                            // pay down the max amount possible for the loan
                            let repay_asset =
                                asset.info.with_balance(cmp::min(asset.amount, max_repay));
                            let repay_loan_msg = capapult_market.repay_loan(repay_asset)?;

                            // this is done through authz as we need to pay from the user wallet
                            msgs.push(repay_loan_msg.to_authz_msg(user, &env)?)
                        },
                    }
                },
                DestinationRuntime::DepositCollateral {
                    market,
                } => {
                    attrs.push(attr("type", "deposit_collateral"));
                    match market {
                        DepositMarket::Capapult {
                            asset_info,
                        } => {
                            attrs.push(attr("market", "capapult"));

                            let capa = state.capapult.load(deps.storage)?;

                            // send fees and rest of the funds back to the user
                            let asset = pay_fees_and_send_to_receiver(
                                &deps, &env, &state, &mut msgs, &mut attrs, asset_info, executor,
                                &user,
                                // in case of capa, it can only execute the deposit for the user
                                &user,
                            )?;

                            // top up the collateral in capapult (increase allowance + lock_collateral)
                            let capapult_locker = CapapultLocker {
                                overseer: capa.overseer,
                                custody: capa.custody,
                            };
                            let deposit_collateral_msgs =
                                capapult_locker.deposit_and_lock_collateral(asset)?;

                            msgs.push(
                                deposit_collateral_msgs.to_authz_msg(user.to_string(), &env)?,
                            );
                        },
                    }
                },
            };

            state.is_executing.remove(deps.storage);
        },
    }

    Ok(Response::new()
        .add_attribute("action", "ampz/callback")
        .add_attributes(attrs)
        .add_messages(msgs))
}

#[allow(clippy::too_many_arguments)]
fn pay_fees_and_send_to_receiver(
    deps: &DepsMut,
    env: &Env,
    state: &State,
    msgs: &mut Vec<CosmosMsg>,
    attrs: &mut Vec<Attribute>,
    asset_info: AssetInfo,
    executor: Addr,
    user: &Addr,
    receiver: &Addr,
) -> Result<Asset, ContractError> {
    // this method, queries the contract for the expected asset, pays fees on it and sends it to the receiver.
    let amount = asset_info.query_pool(&deps.querier, env.contract.address.to_string())?;
    if amount.is_zero() {
        return Err(ContractError::NothingToDeposit {});
    }
    let balances = vec![asset_info.with_balance(amount)];
    let mut balances = pay_fees(state, deps, msgs, attrs, balances, executor, user)?;
    let balance = balances.remove(0);
    msgs.push(balance.transfer_msg(receiver)?);

    Ok(balance)
}

fn pay_fees(
    state: &State,
    deps: &DepsMut,
    msgs: &mut Vec<CosmosMsg>,
    attrs: &mut Vec<Attribute>,
    balances: Vec<Asset>,
    executor: Addr,
    user: &Addr,
) -> StdResult<Vec<Asset>> {
    let fee = state.fee.load(deps.storage)?;

    // when the user is doing manual executions, no operator fee needs to be paid.
    let operator_bps = if *user == executor {
        BasicPoints::zero()
    } else {
        fee.operator_bps
    };

    let total_fee_bps = operator_bps.checked_add(fee.fee_bps)?;
    // when no total fee, nothing needs to be paid
    if total_fee_bps.is_zero() {
        add_balances_to_attributes(&balances, attrs);
        return Ok(balances);
    }

    let controller = state.controller.load(deps.storage)?;

    let mut result: Vec<Asset> = vec![];

    for asset in balances {
        if !asset.amount.is_zero() {
            let mut operator_fee_amount = asset.amount * operator_bps.decimal();
            let mut protocol_fee_amount = asset.amount * fee.fee_bps.decimal();

            let deposit_amount =
                asset.amount.checked_sub(operator_fee_amount)?.checked_sub(protocol_fee_amount)?;

            let deposit_asset = asset.info.with_balance(deposit_amount);

            if executor == fee.receiver || executor == controller {
                protocol_fee_amount += operator_fee_amount;
                operator_fee_amount = Uint128::zero();
            }

            result.push(deposit_asset);

            if !protocol_fee_amount.is_zero() {
                // pay protocol fee
                let protocol_fee = asset.info.with_balance(protocol_fee_amount);
                msgs.push(protocol_fee.transfer_msg(&fee.receiver)?);
                attrs.push(attr("fee", protocol_fee.to_string()));
            }
            if !operator_fee_amount.is_zero() {
                // pay operator fee
                let operator_fee = asset.info.with_balance(operator_fee_amount);
                msgs.push(operator_fee.transfer_msg(&executor)?);
                attrs.push(attr("operator_fee", operator_fee.to_string()));
            }
        }
    }

    add_balances_to_attributes(&result, attrs);

    // return the assets without the fees
    Ok(result)
}

fn add_balances_to_attributes(balances: &[Asset], attrs: &mut Vec<Attribute>) {
    for asset in balances.iter() {
        attrs.push(attr("amount", asset.to_string()));
    }
}

fn deposit_in_farm(
    deps: &DepsMut,
    farm: String,
    env: &Env,
    receiver: String,
    balances: Vec<Asset>,
    msgs: &mut Vec<CosmosMsg>,
) -> Result<(), StdError> {
    let farm = deps.api.addr_validate(&farm)?;
    let (funds, mut allowances) = funds_or_allowance(env, &farm, &balances, None)?;
    msgs.append(&mut allowances);
    msgs.push(Farm(farm).bond_assets_msg(balances, funds, Some(receiver))?);
    Ok(())
}
