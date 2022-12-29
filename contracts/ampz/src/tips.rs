// use astroport::asset::native_asset;
// use cosmwasm_std::{Addr, DepsMut, MessageInfo, Response, StdError, StdResult, Storage, Uint128};

// use crate::{constants::CONTRACT_DENOM, state::State};

// pub fn add_to_tip_jar(deps: DepsMut, info: MessageInfo, recipient: Addr) -> StdResult<Response> {
//     let state = State::default();

//     for fund in info.funds {
//         if fund.denom == CONTRACT_DENOM {
//             add_to_tip_jar_state(&state, deps.storage, &recipient, fund.amount)?;
//         } else {
//             return Err(StdError::generic_err(format!("unsupported tip denom {}", fund.denom)));
//         }
//     }

//     Ok(Response::new().add_attribute("action", "add_to_tip_jar"))
// }

// pub fn withdraw_tip_jar(
//     deps: DepsMut,
//     info: MessageInfo,
//     amount: Option<Uint128>,
// ) -> StdResult<Response> {
//     let state = State::default();

//     let amount = withdraw_from_tip_jar_state(&state, deps.storage, &info.sender, amount)?;

//     if amount.is_zero() {
//         return Err(StdError::generic_err("no funds in tip jar."));
//     }

//     let withdraw_msg =
//         native_asset(CONTRACT_DENOM.to_string(), amount).into_msg(&deps.querier, info.sender)?;

//     Ok(Response::new().add_message(withdraw_msg).add_attribute("action", "withdraw_tip_jar"))
// }

// fn add_to_tip_jar_state(
//     state: &State,
//     storage: &mut dyn Storage,
//     recipient: &Addr,
//     add_amount: Uint128,
// ) -> StdResult<()> {
//     let amount = state.tip_jar.load(storage, recipient.to_string()).unwrap_or_default();
//     let new_jar_amount = amount.checked_add(add_amount)?;
//     state.tip_jar.save(storage, recipient.to_string(), &new_jar_amount)?;

//     Ok(())
// }

// fn withdraw_from_tip_jar_state(
//     state: &State,
//     storage: &mut dyn Storage,
//     taker: &Addr,
//     take_amount: Option<Uint128>,
// ) -> StdResult<Uint128> {
//     let amount = state.tip_jar.load(storage, taker.to_string()).unwrap_or_default();

//     if let Some(take_amount) = take_amount {
//         let new_jar_amount = amount.checked_sub(take_amount)?;

//         if new_jar_amount.is_zero() {
//             state.tip_jar.remove(storage, taker.to_string());
//         } else {
//             state.tip_jar.save(storage, taker.to_string(), &new_jar_amount)?;
//         }
//         Ok(take_amount)
//     } else {
//         state.tip_jar.remove(storage, taker.to_string());
//         Ok(amount)
//     }
// }
