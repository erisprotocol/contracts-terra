use std::vec;

use astroport::asset::{native_asset_info, Asset, AssetInfoExt};
use cosmwasm_std::{
    attr, Addr, Attribute, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult,
};

use crate::constants::CONTRACT_DENOM;
use crate::protos::msgex::CosmosMsgEx;
use crate::state::State;
use eris::adapters::ampz::Ampz;
use eris::adapters::asset::{AssetEx, AssetInfosEx, AssetsEx};
use eris::adapters::farm::Farm;
use eris::ampz::{CallbackMsg, CallbackWrapper};
use eris::helper::funds_or_allowance;
use eris::helpers::bps::BasicPoints;
use itertools::Itertools;

pub fn callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    callback_wrapper: CallbackWrapper,
) -> StdResult<Response> {
    if env.contract.address != info.sender {
        return Err(StdError::generic_err("callbacks can only be invoked by the contract itself"));
    }

    let state = State::default();
    let user = callback_wrapper.user;

    let mut msgs: Vec<CosmosMsg> = vec![];
    let mut attrs: Vec<Attribute> = vec![];

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

            // this swaps all specified assets to the "into" asset.
            let asset_infos = asset_infos.into_iter().filter(|info| *info != into).collect_vec();
            let balances = asset_infos.query_balances(&deps.querier, &env.contract.address)?;
            let zapper = state.zapper.load(deps.storage)?;

            let (funds, mut allowances) = funds_or_allowance(&env, &zapper.0, &balances, None)?;

            for asset in balances.iter() {
                attrs.push(attr("from", asset.to_string()));
            }

            // it uses the ERIS zapper multi-swap feature
            msgs.append(&mut allowances);
            msgs.push(zapper.multi_swap_msg(balances, into.clone(), funds, None)?);
            attrs.push(attr("to", format!("{:?}", into)));
        },

        CallbackMsg::FinishExecution {
            destination,
            asset_infos,
            operator,
        } => {
            match destination {
                eris::ampz::Destination::DepositAmplifier {} => {
                    attrs.push(attr("type", "deposit_amplifier"));
                    let balances =
                        asset_infos.query_balances(&deps.querier, &env.contract.address)?;
                    let balances =
                        pay_fees(&state, &deps, &mut msgs, &mut attrs, balances, operator, &user)?;
                    let main_token = native_asset_info(CONTRACT_DENOM.to_string());
                    let amount = balances
                        .iter()
                        .find(|a| a.info == main_token)
                        .ok_or_else(|| StdError::generic_err("main token not found"))?;

                    let hub = state.hub.load(deps.storage)?;
                    let bond_msg =
                        hub.bond_msg(CONTRACT_DENOM, amount.amount.u128(), Some(user.into()))?;
                    msgs.push(bond_msg);

                    for asset in balances {
                        attrs.push(attr("amount", asset.to_string()));
                    }
                },

                eris::ampz::Destination::DepositFarm {
                    farm,
                } => {
                    attrs.push(attr("type", "deposit_farm"));
                    let balances =
                        asset_infos.query_balances(&deps.querier, &env.contract.address)?;
                    let balances =
                        pay_fees(&state, &deps, &mut msgs, &mut attrs, balances, operator, &user)?;

                    for asset in balances.iter() {
                        attrs.push(attr("amount", asset.to_string()));
                    }

                    deposit_in_farm(&deps, farm, &env, &user, balances, &mut msgs)?;
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

fn pay_fees(
    state: &State,
    deps: &DepsMut,
    msgs: &mut Vec<CosmosMsg>,
    attrs: &mut Vec<Attribute>,
    balances: Vec<Asset>,
    operator: Addr,
    user: &Addr,
) -> StdResult<Vec<Asset>> {
    let fee = state.fee.load(deps.storage)?;

    // when the user is doing manual executions, no operator fee needs to be paid.
    let operator_bps = if *user == operator {
        BasicPoints::zero()
    } else {
        fee.operator_bps
    };

    let total_fee = operator_bps.checked_add(fee.fee_bps)?;
    // when no total fee, nothing needs to be paid
    if total_fee.is_zero() {
        return Ok(balances);
    }

    // share of what the operator will receive
    // if the operator is the treasury, operator will receive 0 and all the treasury (only single transfer)
    let operator_share = if operator == fee.receiver {
        Decimal::zero()
    } else {
        operator_bps.div_decimal(total_fee)
    };

    let mut result: Vec<Asset> = vec![];

    for asset in balances {
        if !asset.amount.is_zero() {
            // split total fees from the asset
            let (deposit_asset, fee_amount) = asset.subtract_fee(total_fee);

            result.push(deposit_asset);

            // split fee into operator and protocol fee
            let operator_fee_amount = fee_amount * operator_share;
            let protocol_fee_amount = fee_amount.saturating_sub(operator_fee_amount);

            if !protocol_fee_amount.is_zero() {
                // pay protocol fee
                let protocol_fee = asset.info.with_balance(protocol_fee_amount);
                msgs.push(protocol_fee.transfer_msg(&fee.receiver)?);
                attrs.push(attr("fee", protocol_fee.to_string()));
            }
            if !operator_fee_amount.is_zero() {
                // pay operator fee
                let operator_fee = asset.info.with_balance(operator_fee_amount);
                msgs.push(operator_fee.transfer_msg(&operator)?);
                attrs.push(attr("operator_fee", operator_fee.to_string()));
            }
        }
    }

    // return the assets without the fees
    Ok(result)
}

fn deposit_in_farm(
    deps: &DepsMut,
    farm: String,
    env: &Env,
    user: &Addr,
    balances: Vec<Asset>,
    msgs: &mut Vec<CosmosMsg>,
) -> Result<(), StdError> {
    let farm = deps.api.addr_validate(&farm)?;
    let (funds, mut allowances) = funds_or_allowance(env, &farm, &balances, None)?;
    msgs.append(&mut allowances);
    msgs.push(Farm(farm).bond_assets_msg(balances, funds, Some(user.into()))?);
    Ok(())
}
