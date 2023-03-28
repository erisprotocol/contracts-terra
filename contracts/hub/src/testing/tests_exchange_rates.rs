use cosmwasm_std::testing::{mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{attr, Coin, Decimal, Uint128};

use eris::constants::DAY;
use eris::hub::{CallbackMsg, ExchangeRatesResponse, ExecuteMsg, QueryMsg};
use eris::DecimalCheckedOps;

use crate::contract::execute;
use crate::state::State;
use crate::testing::helpers::{query_helper_env, setup_test, MOCK_UTOKEN, STAKE_DENOM};
use crate::types::Delegation;

use super::helpers::mock_env_at_timestamp;

//--------------------------------------------------------------------------------------------------
// Execution
//--------------------------------------------------------------------------------------------------

#[test]
fn reinvesting_check_exchange_rates() {
    let mut deps = setup_test();
    let state = State::default();

    deps.querier.set_staking_delegations(&[
        Delegation::new("alice", 333334),
        Delegation::new("bob", 333333),
        Delegation::new("charlie", 333333),
    ]);

    // After the swaps, `unlocked_coins` should contain only utoken and unknown denoms
    state.unlocked_coins.save(deps.as_mut().storage, &vec![Coin::new(234, MOCK_UTOKEN)]).unwrap();

    deps.querier.set_cw20_total_supply(STAKE_DENOM, 100000);

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(0),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::Callback(CallbackMsg::Reinvest {}),
    )
    .unwrap();
    assert_eq!(res.messages.len(), 2);

    // ustake: (0_100000 - (111 - fees)), utoken: 1_000000 + (234 - fees)
    assert_eq!(
        res.attributes,
        vec![attr("action", "erishub/reinvest"), attr("exchange_rate", "10.00232")]
    );

    // added delegation of 234 - fees
    let total = Uint128::from(234u128);
    let fee = Decimal::from_ratio(1u128, 100u128).checked_mul_uint(total).unwrap();
    let delegated = total.saturating_sub(fee);
    deps.querier.set_staking_delegations(&[
        Delegation::new("alice", 333334),
        Delegation::new("bob", 333333 + delegated.u128()),
        Delegation::new("charlie", 333333),
    ]);

    state.unlocked_coins.save(deps.as_mut().storage, &vec![Coin::new(200, MOCK_UTOKEN)]).unwrap();

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::Callback(CallbackMsg::Reinvest {}),
    )
    .unwrap();
    assert_eq!(res.messages.len(), 2);

    let res: ExchangeRatesResponse = query_helper_env(
        deps.as_ref(),
        QueryMsg::ExchangeRates {
            start_after: None,
            limit: None,
        },
        2083600,
    );
    assert_eq!(
        res.exchange_rates
            .into_iter()
            .map(|a| format!("{0};{1}", a.0, a.1))
            .collect::<Vec<String>>(),
        vec!["86400;10.0043".to_string(), "0;10.00232".to_string()]
    );

    // 10.00232 -> 10.0043 within 1 day
    assert_eq!(res.apr.map(|a| a.to_string()), Some("0.00019795407465468".to_string()));
}
