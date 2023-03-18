use astroport::asset::{token_asset, token_asset_info};
use cosmwasm_std::{attr, coin, to_binary, Decimal, StdResult, Uint128};
use eris::constants::DAY;
use eris::governance_helper::WEEK;
use eris_arb_vault::error::ContractError;
use eris_tests::gov_helper::EscrowHelper;
use eris_tests::{mock_app, EventChecker, TerraAppExtension};
use std::str::FromStr;
use std::vec;

use eris::arb_vault::{
    Balances, ClaimBalance, Config, ConfigResponse, ExecuteMsg, StateResponse, UserInfoResponse,
    UtilizationMethod,
};

#[test]
fn update_config() -> StdResult<()> {
    let mut router = mock_app();
    let helper = EscrowHelper::init(&mut router, false);

    let config = helper.arb_query_config(&mut router).unwrap();

    assert_eq!(
        config.config.utilization_method,
        UtilizationMethod::Steps(vec![
            (dec("0.010"), dec("0.5")),
            (dec("0.015"), dec("0.7")),
            (dec("0.020"), dec("0.9")),
            (dec("0.025"), dec("1.0")),
        ])
    );

    let result = helper
        .arb_execute_sender(
            &mut router,
            ExecuteMsg::UpdateConfig {
                utilization_method: Some(UtilizationMethod::Steps(vec![(dec("0.1"), dec("1"))])),
                unbond_time_s: None,
                lsds: None,
                fee_config: None,
                set_whitelist: None,
                remove_whitelist: None,
            },
            "user",
        )
        .unwrap_err();
    assert_eq!("Unauthorized", result.root_cause().to_string());

    helper
        .arb_execute(
            &mut router,
            ExecuteMsg::UpdateConfig {
                utilization_method: Some(UtilizationMethod::Steps(vec![(dec("0.1"), dec("1"))])),
                unbond_time_s: None,
                lsds: None,
                fee_config: None,
                set_whitelist: None,
                remove_whitelist: None,
            },
        )
        .unwrap();

    let config = helper.arb_query_config(&mut router).unwrap();

    assert_eq!(
        config.config.utilization_method,
        UtilizationMethod::Steps(vec![(dec("0.1"), dec("1"))])
    );

    Ok(())
}

#[test]
fn provide_liquidity_and_arb_fails() -> StdResult<()> {
    let mut router = mock_app();
    let router_ref = &mut router;
    let helper = EscrowHelper::init(router_ref, false);

    router_ref.next_block(100);
    helper.hub_bond(router_ref, "user1", 100_000000, "uluna").unwrap();
    helper.arb_fake_fill_arb_contract(router_ref);

    helper.arb_provide_liquidity(router_ref, "user1", 100_000000).unwrap();
    helper.arb_provide_liquidity(router_ref, "user2", 50_000000).unwrap();
    helper.arb_provide_liquidity(router_ref, "user3", 150_000000).unwrap();

    let user = helper.arb_query_user_info(router_ref, "user1").unwrap();
    assert_eq!(
        user,
        UserInfoResponse {
            utoken_amount: Uint128::new(100_000000),
            lp_amount: Uint128::new(100_000000)
        }
    );

    let amount = Uint128::new(10_000000u128);
    let profit_percent = dec("1.02");
    let fee_percent = dec("0.1");
    let absolute_profit = amount * profit_percent - amount;
    let fee = absolute_profit * fee_percent;
    let no_profit_return = dec("1");

    // execute arb
    let res = helper
        .arb_execute(
            router_ref,
            ExecuteMsg::ExecuteArbitrage {
                msg: return_msg(&helper, amount, amount * profit_percent),
                result_token: token_asset_info(helper.get_ustake_addr()),
                wanted_profit: dec("0.01"),
            },
        )
        .unwrap_err();
    assert_eq!(res.root_cause().to_string(), "Unauthorized: Sender not on whitelist");

    let res = helper
        .arb_execute_whitelist(
            router_ref,
            ExecuteMsg::ExecuteArbitrage {
                msg: return_msg(&helper, amount, amount * no_profit_return),
                result_token: token_asset_info(helper.get_ustake_addr()),
                wanted_profit: dec("0.01"),
            },
        )
        .unwrap_err();
    assert_eq!(res.root_cause().to_string(), "Not enough profit");

    Ok(())
}

#[test]
fn provide_liquidity_and_arb() -> StdResult<()> {
    let mut router = mock_app();
    let router_ref = &mut router;
    let helper = EscrowHelper::init(router_ref, false);

    router_ref.next_block(100);
    helper.hub_bond(router_ref, "user1", 100_000000, "uluna").unwrap();
    helper.arb_fake_fill_arb_contract(router_ref);

    helper.arb_provide_liquidity(router_ref, "user1", 100_000000).unwrap();
    helper.arb_provide_liquidity(router_ref, "user2", 50_000000).unwrap();
    helper.arb_provide_liquidity(router_ref, "user3", 150_000000).unwrap();

    let user = helper.arb_query_user_info(router_ref, "user1").unwrap();
    assert_eq!(
        user,
        UserInfoResponse {
            utoken_amount: Uint128::new(100_000000),
            lp_amount: Uint128::new(100_000000)
        }
    );

    let amount = Uint128::new(10_000000u128);
    let profit_percent = dec("1.02");
    let fee_percent = dec("0.1");
    let absolute_profit = amount * profit_percent - amount;
    let fee = absolute_profit * fee_percent;

    // EXECUTE ARB
    let res = helper
        .arb_execute_whitelist(
            router_ref,
            ExecuteMsg::ExecuteArbitrage {
                msg: return_msg(&helper, amount, amount * profit_percent),
                result_token: token_asset_info(helper.get_ustake_addr()),
                wanted_profit: dec("0.01"),
            },
        )
        .unwrap();

    // ASSERT RESULT
    res.assert_attribute("wasm", attr("profit", "200000")).unwrap();
    res.assert_attribute("wasm", attr("exchange_rate", "1.0006")).unwrap();
    res.assert_attribute("wasm-erishub/unbond_queued", attr("ustake_to_burn", "10200000")).unwrap();

    let user = helper.arb_query_user_info(router_ref, "user1").unwrap();
    assert_eq!(
        user,
        UserInfoResponse {
            utoken_amount: uint(100_060000),
            lp_amount: uint(100_000000)
        }
    );

    let state = helper.arb_query_state(router_ref, Some(true)).unwrap();
    assert_eq!(
        state,
        StateResponse {
            exchange_rate: dec("1.0006"),
            total_lp_supply: uint(300_000000),
            balances: Balances {
                tvl_utoken: uint(300_000000) + absolute_profit - fee,
                vault_total: uint(300_000000) + absolute_profit - fee,
                vault_available: uint(300_000000) - amount - fee, // fee is taken from available
                vault_takeable: uint(300_000000) - amount - fee,
                locked_user_withdrawls: uint(0),
                lsd_unbonding: amount * profit_percent,
                lsd_withdrawable: uint(0),
            },
            details: Some(eris::arb_vault::StateDetails {
                claims: vec![ClaimBalance {
                    name: "eris".to_string(),
                    withdrawable: uint(0),
                    unbonding: uint(10200000)
                }],
                takeable_steps: vec![
                    (dec("0.010"), uint(139_890000)),
                    (dec("0.015"), uint(199_926000)),
                    (dec("0.020"), uint(259_962000)),
                    (dec("0.025"), uint(289_980000)),
                ]
            })
        }
    );

    Ok(())
}

#[test]
fn provide_liquidity_and_arb_submit() -> StdResult<()> {
    let mut router = mock_app();
    let router_ref = &mut router;
    let helper = EscrowHelper::init(router_ref, false);

    router_ref.next_block(100);
    helper.hub_bond(router_ref, "user1", 100_000000, "uluna").unwrap();
    helper.arb_fake_fill_arb_contract(router_ref);

    helper.arb_provide_liquidity(router_ref, "user1", 100_000000).unwrap();
    helper.arb_provide_liquidity(router_ref, "user2", 50_000000).unwrap();
    helper.arb_provide_liquidity(router_ref, "user3", 150_000000).unwrap();

    let user = helper.arb_query_user_info(router_ref, "user1").unwrap();
    assert_eq!(
        user,
        UserInfoResponse {
            utoken_amount: Uint128::new(100_000000),
            lp_amount: Uint128::new(100_000000)
        }
    );

    let amount = Uint128::new(10_000000u128);
    let profit_percent = dec("1.02");
    let fee_percent = dec("0.1");
    let absolute_profit = amount * profit_percent - amount;
    let fee = absolute_profit * fee_percent;

    // EXECUTE ARB
    let res = helper
        .arb_execute_whitelist(
            router_ref,
            ExecuteMsg::ExecuteArbitrage {
                msg: return_msg(&helper, amount, amount * profit_percent),
                result_token: token_asset_info(helper.get_ustake_addr()),
                wanted_profit: dec("0.01"),
            },
        )
        .unwrap();

    // SUBMIT BATCH
    router_ref.next_block(DAY * 3);
    helper.hub_submit_batch(router_ref).unwrap();
    router_ref.next_block(DAY * 3);

    // STATE IS STILL THE SAME AS in provide_liquidity_and_arb
    let state = helper.arb_query_state(router_ref, Some(true)).unwrap();
    assert_eq!(
        state,
        StateResponse {
            exchange_rate: dec("1.0006"),
            total_lp_supply: uint(300_000000),
            balances: Balances {
                tvl_utoken: uint(300_000000) + absolute_profit - fee,
                vault_total: uint(300_000000) + absolute_profit - fee,
                vault_available: uint(300_000000) - amount - fee, // fee is taken from available
                vault_takeable: uint(300_000000) - amount - fee,
                locked_user_withdrawls: uint(0),
                lsd_unbonding: amount * profit_percent,
                lsd_withdrawable: uint(0),
            },
            details: Some(eris::arb_vault::StateDetails {
                claims: vec![ClaimBalance {
                    name: "eris".to_string(),
                    withdrawable: uint(0),
                    unbonding: uint(10200000)
                }],
                takeable_steps: vec![
                    (dec("0.010"), uint(139_890000)),
                    (dec("0.015"), uint(199_926000)),
                    (dec("0.020"), uint(259_962000)),
                    (dec("0.025"), uint(289_980000)),
                ]
            })
        }
    );

    router_ref.next_block(DAY * 19);
    helper.hub_reconcile(router_ref, 10200000).unwrap();

    // STATE IS STILL THE SAME AS in provide_liquidity_and_arb only withdrawable has changed
    let state = helper.arb_query_state(router_ref, Some(true)).unwrap();
    assert_eq!(
        state,
        StateResponse {
            exchange_rate: dec("1.0006"),
            total_lp_supply: uint(300_000000),
            balances: Balances {
                tvl_utoken: uint(300_000000) + absolute_profit - fee,
                vault_total: uint(300_000000) + absolute_profit - fee,
                vault_available: uint(300_000000) - amount - fee, // fee is taken from available
                vault_takeable: uint(300_000000) - amount - fee,
                locked_user_withdrawls: uint(0),
                lsd_unbonding: uint(0),
                // moved to withdrawable
                lsd_withdrawable: amount * profit_percent,
            },
            details: Some(eris::arb_vault::StateDetails {
                claims: vec![ClaimBalance {
                    name: "eris".to_string(),
                    // moved to withdrawable
                    withdrawable: uint(10200000),
                    unbonding: uint(0)
                }],
                takeable_steps: vec![
                    (dec("0.010"), uint(139_890000)),
                    (dec("0.015"), uint(199_926000)),
                    (dec("0.020"), uint(259_962000)),
                    (dec("0.025"), uint(289_980000)),
                ]
            })
        }
    );

    // EXECUTE ARB
    let res = helper
        .arb_execute_whitelist(
            router_ref,
            ExecuteMsg::ExecuteArbitrage {
                msg: return_msg(&helper, amount, amount * profit_percent),
                result_token: token_asset_info(helper.get_ustake_addr()),
                wanted_profit: dec("0.01"),
            },
        )
        .unwrap_err();

    assert_eq!(
        res.root_cause().to_string(),
        "Withdrawable funds available. Execute withdraw before arbitrage.".to_string()
    );

    let res =
        helper.arb_execute_whitelist(router_ref, ExecuteMsg::WithdrawFromLiquidStaking {}).unwrap();

    // STATE IS STILL THE SAME AS in provide_liquidity_and_arb this time everything is withdrawn from the lsd vaults
    let state = helper.arb_query_state(router_ref, Some(true)).unwrap();
    assert_eq!(
        state,
        StateResponse {
            exchange_rate: dec("1.0006"),
            total_lp_supply: uint(300_000000),
            balances: Balances {
                tvl_utoken: uint(300_000000) + absolute_profit - fee,
                vault_total: uint(300_000000) + absolute_profit - fee,
                vault_available: uint(300_000000) + absolute_profit - fee, // fee is taken from available
                vault_takeable: uint(300_000000) + absolute_profit - fee,
                locked_user_withdrawls: uint(0),
                lsd_unbonding: uint(0),
                // moved to withdrawable
                lsd_withdrawable: uint(0),
            },
            details: Some(eris::arb_vault::StateDetails {
                claims: vec![ClaimBalance {
                    name: "eris".to_string(),
                    // moved to withdrawable
                    withdrawable: uint(0),
                    unbonding: uint(0)
                }],
                takeable_steps: vec![
                    (dec("0.010"), uint(150090000)),
                    (dec("0.015"), uint(210126000)),
                    (dec("0.020"), uint(270162000)),
                    (dec("0.025"), uint(300180000)),
                ]
            })
        }
    );

    Ok(())
}

fn return_msg(
    helper: &EscrowHelper,
    amount: Uint128,
    return_amount: Uint128,
) -> eris::arb_vault::ExecuteSubMsg {
    eris::arb_vault::ExecuteSubMsg {
        contract_addr: Some(helper.base.arb_fake_contract.get_address_string()),
        msg: to_binary(&eris_tests::arb_contract::ExecuteMsg::ReturnAsset {
            asset: token_asset(helper.get_ustake_addr(), return_amount),
            received: vec![coin(amount.u128(), "uluna")],
        })
        .unwrap(),
        funds_amount: amount,
    }
}

fn uint(val: u128) -> Uint128 {
    Uint128::new(val)
}

fn dec(val: &str) -> Decimal {
    Decimal::from_str(val).unwrap()
}
