// use crate::extensions::ConfigEx;
// use astroport::asset::{token_asset, Asset, AssetInfo};
// use cosmwasm_std::{Addr, Decimal, DepsMut, Env, MessageInfo, Response};
// use eris::helper::addr_opt_validate;

// use crate::{
//     error::{ContractError, ContractResult},
//     state::State,
// };

// #[allow(clippy::too_many_arguments)]
// pub(crate) fn execute_swap_native(
//     deps: DepsMut,
//     env: Env,
//     info: MessageInfo,
//     offer_asset: Asset,
//     ask_asset_info: Option<AssetInfo>,
//     belief_price: Option<Decimal>,
//     max_spread: Option<Decimal>,
//     to: Option<String>,
// ) -> ContractResult {
//     let receiver_addr = addr_opt_validate(deps.api, &to)?;
//     let to_addr = receiver_addr.unwrap_or_else(|| info.sender.clone());

//     offer_asset.info.check(deps.api)?;
//     if !offer_asset.is_native_token() {
//         return Err(ContractError::Cw20DirectSwap {});
//     }
//     offer_asset.assert_sent_native_token_balance(&info)?;

//     execute_swap(deps, env, offer_asset, ask_asset_info, belief_price, max_spread, to_addr)
// }

// #[allow(clippy::too_many_arguments)]
// pub(crate) fn execute_swap_cw20(
//     deps: DepsMut,
//     env: Env,
//     cw20_sender: String,
//     offer_asset: Asset,
//     ask_asset_info: Option<AssetInfo>,
//     belief_price: Option<Decimal>,
//     max_spread: Option<Decimal>,
//     to: Option<String>,
// ) -> ContractResult {
//     let receiver_addr = addr_opt_validate(deps.api, &to)?;
//     let cw20_addr = deps.api.addr_validate(&cw20_sender)?;
//     let to_addr = receiver_addr.unwrap_or(cw20_addr);

//     execute_swap(deps, env, offer_asset, ask_asset_info, belief_price, max_spread, to_addr)
// }

// fn execute_swap(
//     deps: DepsMut,
//     env: Env,
//     offer_asset: Asset,
//     ask_asset_info: Option<AssetInfo>,
//     belief_price: Option<Decimal>,
//     max_spread: Option<Decimal>,
//     to_addr: cosmwasm_std::Addr,
// ) -> ContractResult {
//     let state = State::default();
//     let config = state.config.load(deps.storage)?;
//     let lsds = config.lsd_group(&env);

//     let mut allowed_assets = lsds.;

//     Ok(Response::new().add_attribute("action", "arb/execute_swap"))
// }
