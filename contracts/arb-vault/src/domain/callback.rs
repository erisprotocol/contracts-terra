use std::ops::Div;

use astroport::asset::{native_asset, AssetInfo, AssetInfoExt};
use cosmwasm_std::{attr, Decimal, DepsMut, Env, MessageInfo, Response};
use eris::arb_vault::{CallbackMsg, ExchangeHistory};
use eris::constants::DAY;
use eris::CustomResponse;

use crate::error::{ContractError, ContractResult};
use crate::extensions::{BalancesEx, ConfigEx};
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

    let result_token_string = result_token.to_string();
    let active_lsd_adapter = lsds.get_adapter_by_asset(result_token)?;
    let active_lsd_balance = new_balances.get_by_name(&active_lsd_adapter.name)?;

    let old_value = old_balance.tvl_utoken;
    let new_value = new_balances.tvl_utoken;

    let used_balance = old_balance
        .vault_available
        .checked_sub(new_balances.vault_available)
        .map_err(|e| ContractError::CalculationError("used balance".into(), e.to_string()))?;

    let profit = new_value
        .checked_sub(old_value)
        .map_err(|e| ContractError::CalculationError("profit".into(), e.to_string()))?;

    let received_x_amount = active_lsd_balance
        .xbalance
        .checked_sub(old_balance.active_balance.xbalance)
        .map_err(|e| ContractError::CalculationError("profit_by_asset".into(), e.to_string()))?;
    let profit_by_xasset = received_x_amount * active_lsd_balance.xfactor - used_balance;

    if profit < profit_by_xasset {
        // profit over all assets should be higher than the profit by asset only.
        // profit = profit over all assets
        // profit_by_asset = profit only from the added amount of xasset
        return Err(ContractError::ProfitBalancesDoesNotMatch {
            profit,
            profit_by_xasset,
            old_balance: old_balance.active_balance.xbalance,
        });
    }

    // we allow for a fixed 10 % lower profit than wanted -> still minimum profit at 0.45 %
    // this can be seen as some allowed slippage
    let profit_percentage = Decimal::from_ratio(profit, used_balance);
    let min_profit_percent = wanted_profit * Decimal::percent(90);

    if profit_percentage < min_profit_percent {
        return Err(ContractError::NotEnoughProfit {});
    }

    if new_balances.vault_available < new_balances.locked_user_withdrawls {
        // if locked balance bigger than the available balance, no arbitrage can be executed, as funds are marked for unbond
        return Err(ContractError::DoNotTakeLockedBalance {});
    }

    // calculate fee
    let fee_config = state.fee_config.load(deps.storage)?;
    let fee_percent = fee_config.protocol_performance_fee;
    let fee_amount = profit * fee_percent;

    let (fee_msg, fee_attributes) = if fee_amount.is_zero() {
        (None, vec![])
    } else if new_balances.vault_takeable >= fee_amount {
        // send fees in utoken if takeable allows it.
        let utoken = native_asset(config.utoken, fee_amount);
        let fee_msg = utoken.into_msg(&deps.querier, fee_config.protocol_fee_contract)?;

        (Some(fee_msg), vec![])
    } else {
        // send fees in xtoken otherwise
        let fee_xamount = fee_amount * Decimal::one().div(active_lsd_balance.xfactor);

        let fee_msg = active_lsd_adapter
            .adapter
            .asset()
            .with_balance(fee_xamount)
            .into_msg(&deps.querier, fee_config.protocol_fee_contract)?;

        (
            Some(fee_msg),
            vec![
                attr("fee_xamount", fee_xamount),
                attr("fee_xfactor", active_lsd_balance.xfactor.to_string()),
            ],
        )
    };

    state.balance_checkpoint.remove(deps.storage);

    // we store the exchange rate daily to not create too much data.
    let new_vault_total = new_balances.vault_total - fee_amount;

    let exchange_rate = Decimal::from_ratio(new_vault_total, total_lp_supply);
    state.exchange_history.save(
        deps.storage,
        env.block.time.seconds().div(DAY),
        &ExchangeHistory {
            exchange_rate,
            time_s: env.block.time.seconds(),
        },
    )?;

    Ok(Response::new()
        .add_optional_message(fee_msg)
        .add_attributes(vec![
            attr("action", "arb/assert_result"),
            attr("type", active_lsd_adapter.name.clone()),
            attr("result_token", result_token_string),
            attr("received_xamount", received_x_amount),
            attr("old_tvl", old_value),
            attr("new_tvl", new_value),
            attr("used_balance", used_balance),
            attr("profit", profit),
            attr("exchange_rate", exchange_rate.to_string()),
            attr("fee_amount", fee_amount),
        ])
        .add_attributes(fee_attributes))
}
