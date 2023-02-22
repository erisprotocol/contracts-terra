use std::ops::Div;

use astroport::asset::{native_asset, AssetInfo, AssetInfoExt};
use cosmwasm_std::{attr, Decimal, DepsMut, Env, MessageInfo, Response};
use eris::arb_vault::CallbackMsg;
use eris::constants::DAY;

use crate::error::{ContractError, ContractResult};
use crate::extensions::ConfigEx;
use crate::state::State;

pub fn handle_callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    callback_wrapper: CallbackMsg,
) -> ContractResult {
    if env.contract.address != info.sender {
        return Err(ContractError::CallbackOnlyCalledByContract {});
    }

    match callback_wrapper {
        CallbackMsg::AssertResult {
            result_token,
            wanted_profit,
        } => execute_assert_result(deps, env, result_token, wanted_profit),
    }
}

//----------------------------------------------------------------------------------------
//  PRIVATE FUNCTIONS
//----------------------------------------------------------------------------------------
pub fn execute_assert_result(
    deps: DepsMut,
    env: Env,
    result_token: AssetInfo,
    wanted_profit: Decimal,
) -> ContractResult {
    let state = State::default();
    let config = state.config.load(deps.storage)?;
    let mut lsds = config.lsd_group(&env);

    let old_balance = state.assert_is_nested(deps.storage)?;
    let new_balances = lsds.get_total_assets_err(deps.as_ref(), &env, &state, &config)?;
    let total_lp_supply = config.query_lp_supply(&deps.querier)?;

    let lsd_adapter = lsds.get(result_token)?;

    // get the new
    let xbalance = lsd_adapter.asset().query_pool(&deps.querier, env.contract.address)?;
    let xfactor = lsd_adapter.query_factor_x_to_normal(&deps.as_ref())?;
    let xvalue = xbalance * xfactor;

    let old_value = old_balance.tvl_utoken;
    let new_value = new_balances.tvl_utoken.checked_add(xvalue)?;

    let used_balance = old_balance
        .vault_available
        .checked_sub(new_balances.vault_available)
        .map_err(|e| ContractError::CalculationError("used balance".into(), e.to_string()))?;

    let profit = new_value
        .checked_sub(old_value)
        .map_err(|e| ContractError::CalculationError("profit".into(), e.to_string()))?;

    // we allow for a fixed 10 % lower profit than wanted -> still minimum profit at 0.45 %
    // this can be seen as some allowed slippage
    let profit_percentage = Decimal::from_ratio(profit, used_balance);
    let min_profit_percent = wanted_profit * Decimal::percent(90);

    // println!("Old-Assets: {:?}", old_assets);
    // println!("X-Factor: {:?}", xfactor.to_string());
    // println!("X-Value: {:?}", xvalue);
    // println!("X-Balance: {:?}", xbalance);
    // println!("New-Value: {:?}", new_value);
    // println!("New-Assets: {:?}", new_assets);
    // println!("Used-Balance: {:?}", used_balance);
    // println!("Profit: {}", profit_percentage);
    // println!("Min-Profit: {}", min_profit_percent);

    if xbalance.is_zero() {
        return Err(ContractError::NothingToUnbond {});
    }

    if profit_percentage < min_profit_percent {
        return Err(ContractError::NotEnoughProfit {});
    }

    if new_balances.vault_available < new_balances.locked_user_withdrawls {
        // if locked balance bigger than the available balance, no arbitrage can be executed, as funds are marked for unbond
        return Err(ContractError::DoNotTakeLockedBalance {});
    }

    let mut unbond_xamount = xbalance;

    // calculate fee
    let fee_config = state.fee_config.load(deps.storage)?;
    let fee_percent = fee_config.protocol_performance_fee;
    let fee_amount = profit * fee_percent;

    let (fee_msg, fee_attribute) = if new_balances.vault_takeable >= fee_amount {
        // send fees in utoken if takeable allows it.
        let utoken = native_asset(config.utoken, fee_amount);
        let fee_msg = utoken.into_msg(&deps.querier, fee_config.protocol_fee_contract)?;

        (fee_msg, attr("fee_amount", fee_amount))
    } else {
        // send fees in xtoken otherwise
        let fee_xamount = fee_amount * Decimal::one().div(xfactor);
        unbond_xamount = unbond_xamount.checked_sub(fee_xamount)?;

        let fee_msg = lsd_adapter
            .asset()
            .with_balance(fee_xamount)
            .into_msg(&deps.querier, fee_config.protocol_fee_contract)?;

        (fee_msg, attr("fee_xamount", fee_xamount))
    };

    state.balance_checkpoint.remove(deps.storage);

    // we store the exchange rate daily to not create too much data.
    let exchange_rate = Decimal::from_ratio(new_balances.vault_total, total_lp_supply);
    state.exchange_history.save(deps.storage, env.block.time.seconds().div(DAY), &exchange_rate)?;

    return Ok(Response::new()
        .add_messages(lsd_adapter.unbond(&deps.as_ref(), unbond_xamount)?)
        .add_message(fee_msg)
        .add_attributes(vec![
            attr("action", "arb/assert_result"),
            attr("type", lsd_adapter.get_name()),
            attr("old_value", old_value),
            attr("new_value", new_value),
            attr("used_balance", used_balance),
            attr("xbalance", xbalance),
            attr("unbond_xamount", unbond_xamount),
            attr("xfactor", xfactor.to_string()),
            attr("xvalue", xvalue),
            attr("profit", profit),
            attr("exchange_rate", exchange_rate.to_string()),
        ])
        .add_attributes(vec![fee_attribute]));
}
