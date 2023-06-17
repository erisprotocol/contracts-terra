use astroport::asset::{token_asset, token_asset_info};
use cosmwasm_std::{attr, coin, to_binary, Decimal, StdResult, Uint128};
use eris::constants::DAY;
use eris_tests::gov_helper::EscrowHelper;
use eris_tests::{mock_app, EventChecker, TerraAppExtension};
use std::ops::Div;
use std::str::FromStr;
use std::vec;

use eris::arb_vault::{Balances, ClaimBalance, ExecuteMsg, StateResponse, UserInfoResponse};

#[test]
fn provide_liquidity_and_arb_fails() -> StdResult<()> {
    let mut router = mock_app();
    let router_ref = &mut router;
    let helper = EscrowHelper::init(router_ref, false);
    helper.arb_remove_eris_lsd(router_ref).unwrap();
    helper.arb_stader_insert(router_ref).unwrap();

    router_ref.next_block(100);
    helper.stader_bond(router_ref, "user1", 100_000000, "uluna").unwrap();
    helper.arb_stader_fake_fill_arb_contract(router_ref);

    helper.arb_deposit(router_ref, "user1", 100_000000).unwrap();
    helper.arb_deposit(router_ref, "user2", 50_000000).unwrap();
    helper.arb_deposit(router_ref, "user3", 150_000000).unwrap();

    let user = helper.arb_query_user_info(router_ref, "user1").unwrap();
    assert_eq!(
        user,
        UserInfoResponse {
            utoken_amount: uint(100_000000),
            lp_amount: uint(100_000000)
        }
    );

    let amount = uint(10_000000u128);
    let profit_percent = dec("1.02");
    let no_profit_return = dec("1");

    // execute arb
    let res = helper
        .arb_execute(
            router_ref,
            ExecuteMsg::ExecuteArbitrage {
                msg: return_msg(&helper, amount, amount * profit_percent),
                result_token: token_asset_info(helper.get_stader_token_addr()),
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
                result_token: token_asset_info(helper.get_stader_token_addr()),
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
    helper.arb_remove_eris_lsd(router_ref).unwrap();
    helper.arb_stader_insert(router_ref).unwrap();

    router_ref.next_block(100);
    helper.stader_bond(router_ref, "user1", 100_000000, "uluna").unwrap();
    helper.arb_stader_fake_fill_arb_contract(router_ref);

    helper.arb_deposit(router_ref, "user1", 100_000000).unwrap();
    helper.arb_deposit(router_ref, "user2", 50_000000).unwrap();
    helper.arb_deposit(router_ref, "user3", 150_000000).unwrap();

    let user = helper.arb_query_user_info(router_ref, "user1").unwrap();
    assert_eq!(
        user,
        UserInfoResponse {
            utoken_amount: uint(100_000000),
            lp_amount: uint(100_000000)
        }
    );

    let amount = uint(10_000000u128);
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
                result_token: token_asset_info(helper.get_stader_token_addr()),
                wanted_profit: dec("0.01"),
            },
        )
        .unwrap();

    // ASSERT RESULT
    res.assert_attribute("wasm", attr("profit", "200000")).unwrap();
    res.assert_attribute("wasm", attr("exchange_rate", "1.0006")).unwrap();

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
                lsd_unbonding: uint(0),
                lsd_withdrawable: uint(0),
                lsd_xvalue: amount * profit_percent,
                details: Some(vec![ClaimBalance {
                    name: "stader".to_string(),
                    withdrawable: uint(0),
                    unbonding: uint(0),
                    xbalance: uint(10200000),
                    xfactor: dec("1")
                }])
            },
            details: Some(eris::arb_vault::StateDetails {
                takeable_steps: vec![
                    (dec("0.010"), uint(139_890000)),
                    (dec("0.015"), uint(199_926000)),
                    (dec("0.020"), uint(259_962000)),
                    (dec("0.025"), uint(289_980000)),
                ]
            })
        }
    );

    helper
        .arb_execute_whitelist(
            router_ref,
            ExecuteMsg::UnbondFromLiquidStaking {
                names: None,
            },
        )
        .unwrap();

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
                lsd_xvalue: uint(0),
                details: Some(vec![ClaimBalance {
                    name: "stader".to_string(),
                    withdrawable: uint(0),
                    // moved to unbonding
                    unbonding: uint(10200000),
                    xbalance: uint(0),
                    xfactor: dec("1")
                }])
            },
            details: Some(eris::arb_vault::StateDetails {
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

    helper.arb_remove_eris_lsd(router_ref).unwrap();
    helper.arb_stader_insert(router_ref).unwrap();
    router_ref.next_block(100);
    helper.stader_bond(router_ref, "user1", 100_000000, "uluna").unwrap();
    // increase exchange_rate
    helper.stader_donate(router_ref, "user1", 10_000000, "uluna").unwrap();
    helper.arb_stader_fake_fill_arb_contract(router_ref);

    helper.arb_deposit(router_ref, "user1", 100_000000).unwrap();
    helper.arb_deposit(router_ref, "user2", 50_000000).unwrap();
    helper.arb_deposit(router_ref, "user3", 150_000000).unwrap();

    let user = helper.arb_query_user_info(router_ref, "user1").unwrap();
    assert_eq!(
        user,
        UserInfoResponse {
            utoken_amount: uint(100_000000),
            lp_amount: uint(100_000000)
        }
    );

    let amount = uint(10_000000u128);
    let profit_percent = dec("1.02");
    let eris_exchange_rate = dec("1.1");
    let fee_percent = dec("0.1");
    let absolute_profit = amount * profit_percent - amount - uint(1);
    let fee = absolute_profit * fee_percent;

    // EXECUTE ARB
    helper
        .arb_execute_whitelist(
            router_ref,
            ExecuteMsg::ExecuteArbitrage {
                msg: return_msg(&helper, amount, amount * profit_percent.div(eris_exchange_rate)),
                result_token: token_asset_info(helper.get_stader_token_addr()),
                wanted_profit: dec("0.01"),
            },
        )
        .unwrap();

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
                vault_available: uint(300_000000) - amount - fee,
                vault_takeable: uint(300_000000) - amount - fee,
                locked_user_withdrawls: uint(0),
                lsd_unbonding: uint(0),
                lsd_withdrawable: uint(0),
                lsd_xvalue: amount * profit_percent - uint(1),
                details: Some(vec![ClaimBalance {
                    name: "stader".to_string(),
                    withdrawable: uint(0),
                    unbonding: uint(0),
                    xbalance: uint(9272727),
                    xfactor: eris_exchange_rate
                }])
            },
            details: Some(eris::arb_vault::StateDetails {
                takeable_steps: vec![
                    (dec("0.010"), uint(139_890001)),
                    (dec("0.015"), uint(199_926001)),
                    (dec("0.020"), uint(259_962001)),
                    (dec("0.025"), uint(289_980001)),
                ]
            })
        }
    );

    // MOVE FUNDS TO UNBONDING
    helper
        .arb_execute_whitelist(
            router_ref,
            ExecuteMsg::UnbondFromLiquidStaking {
                names: Some(vec!["stader".to_string()]),
            },
        )
        .unwrap();

    // STATE MOVED TO UNBONDING
    let state = helper.arb_query_state(router_ref, Some(true)).unwrap();
    assert_eq!(
        state,
        StateResponse {
            exchange_rate: dec("1.0006"),
            total_lp_supply: uint(300_000000),
            balances: Balances {
                tvl_utoken: uint(300_000000) + absolute_profit - fee,
                vault_total: uint(300_000000) + absolute_profit - fee,
                vault_available: uint(300_000000) - amount - fee,
                vault_takeable: uint(300_000000) - amount - fee,
                locked_user_withdrawls: uint(0),
                // moved to unbonding
                lsd_unbonding: amount * profit_percent - uint(1),
                lsd_withdrawable: uint(0),
                lsd_xvalue: uint(0),
                details: Some(vec![ClaimBalance {
                    name: "stader".to_string(),
                    withdrawable: uint(0),
                    unbonding: uint(10199999),
                    xbalance: uint(0),
                    xfactor: eris_exchange_rate
                }])
            },
            details: Some(eris::arb_vault::StateDetails {
                takeable_steps: vec![
                    (dec("0.010"), uint(139_890001)),
                    (dec("0.015"), uint(199_926001)),
                    (dec("0.020"), uint(259_962001)),
                    (dec("0.025"), uint(289_980001)),
                ]
            })
        }
    );

    // WAIT SOME DAYS BEFORE START TO UNBONDING
    router_ref.next_block(DAY * 3);

    // SUBMIT BATCH
    helper.stader_submit_batch(router_ref).unwrap();
    router_ref.next_block(DAY * 3);

    // STATE IS STILL THE SAME AS in provide_liquidity_and_arb
    let state = helper.arb_query_state(router_ref, Some(true)).unwrap();
    assert_eq!(
        state,
        StateResponse {
            // rounding when unbonding, some is kept in the amplifier
            exchange_rate: dec("1.0006"),
            total_lp_supply: uint(300_000000),
            balances: Balances {
                tvl_utoken: uint(300_000000) + absolute_profit - fee,
                vault_total: uint(300_000000) + absolute_profit - fee,
                vault_available: uint(300_000000) - amount - fee,
                vault_takeable: uint(300_000000) - amount - fee,
                locked_user_withdrawls: uint(0),
                lsd_unbonding: amount * profit_percent - uint(1),
                lsd_withdrawable: uint(0),
                lsd_xvalue: uint(0),
                details: Some(vec![ClaimBalance {
                    name: "stader".to_string(),
                    withdrawable: uint(0),
                    unbonding: uint(10200000) - uint(1),
                    xbalance: uint(0),
                    // rounding increased exchange rate in the amplifier
                    xfactor: dec("1.1")
                }])
            },
            details: Some(eris::arb_vault::StateDetails {
                takeable_steps: vec![
                    (dec("0.010"), uint(139_890001)),
                    (dec("0.015"), uint(199_926001)),
                    (dec("0.020"), uint(259_962001)),
                    (dec("0.025"), uint(289_980001)),
                ]
            })
        }
    );

    router_ref.next_block(DAY * 19);
    helper.stader_reconcile(router_ref, 10200000).unwrap();

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
                vault_available: uint(300_000000) - amount - fee,
                vault_takeable: uint(300_000000) - amount - fee,
                locked_user_withdrawls: uint(0),
                lsd_unbonding: uint(0),
                lsd_withdrawable: amount * profit_percent - uint(1),
                lsd_xvalue: uint(0),
                details: Some(vec![ClaimBalance {
                    name: "stader".to_string(),
                    withdrawable: uint(10200000) - uint(1),
                    unbonding: uint(0),
                    xbalance: uint(0),
                    xfactor: dec("1.1")
                }])
            },
            details: Some(eris::arb_vault::StateDetails {
                takeable_steps: vec![
                    (dec("0.010"), uint(139_890001)),
                    (dec("0.015"), uint(199_926001)),
                    (dec("0.020"), uint(259_962001)),
                    (dec("0.025"), uint(289_980001)),
                ]
            })
        }
    );

    helper
        .arb_execute_whitelist(
            router_ref,
            ExecuteMsg::WithdrawFromLiquidStaking {
                names: None,
            },
        )
        .unwrap();

    // STATE IS STILL THE SAME AS in provide_liquidity_and_arb this time everything is withdrawn from the lsd vaults
    let state = helper.arb_query_state(router_ref, Some(true)).unwrap();
    assert_eq!(
        state,
        StateResponse {
            // because reconcile received more uluna than expected -> rounding correct again
            exchange_rate: dec("1.0006"),
            total_lp_supply: uint(300_000000),
            balances: Balances {
                tvl_utoken: uint(300_000000) + absolute_profit - fee,
                vault_total: uint(300_000000) + absolute_profit - fee,
                vault_available: uint(300_000000) + absolute_profit - fee, // fee is taken from available
                vault_takeable: uint(300_000000) + absolute_profit - fee,
                locked_user_withdrawls: uint(0),
                lsd_unbonding: uint(0),
                lsd_withdrawable: uint(0),
                lsd_xvalue: uint(0),
                details: Some(vec![ClaimBalance {
                    name: "stader".to_string(),
                    withdrawable: uint(0),
                    unbonding: uint(0),
                    xbalance: uint(0),
                    xfactor: dec("1.1")
                }])
            },
            details: Some(eris::arb_vault::StateDetails {
                takeable_steps: vec![
                    (dec("0.010"), uint(150090000)),
                    (dec("0.015"), uint(210126000)),
                    (dec("0.020"), uint(270162000)),
                    (dec("0.025"), uint(300180000)),
                ]
            })
        }
    );

    check_normal_withdraw(router_ref, &helper);
    check_immediate_withdraw(router_ref, helper);

    Ok(())
}

fn check_normal_withdraw(router_ref: &mut cw_multi_test::App, helper: &EscrowHelper) {
    let balance = router_ref.wrap().query_balance("user2", "uluna").unwrap();
    let res = helper.arb_unbond(router_ref, "user2", 50_000000, None).unwrap();
    let balance2 = router_ref.wrap().query_balance("user2", "uluna").unwrap();

    res.assert_attribute("wasm", attr("burnt_amount", "50000000")).unwrap();
    res.assert_attribute("wasm", attr("withdraw_amount", "50030000")).unwrap();
    res.assert_attribute("wasm", attr("receive_amount", "49529700")).unwrap();
    res.assert_attribute("wasm", attr("protocol_fee", "500300")).unwrap();
    assert_eq!(balance.amount, balance2.amount);

    router_ref.next_block(DAY * 25);

    let balance3 = router_ref.wrap().query_balance("user2", "uluna").unwrap();
    assert_eq!(balance3.amount, balance2.amount);
    helper.arb_withdraw(router_ref, "user2").unwrap();
    let balance4 = router_ref.wrap().query_balance("user2", "uluna").unwrap();
    assert_eq!(balance3.amount + uint(49529700), balance4.amount);
}

fn check_immediate_withdraw(router_ref: &mut cw_multi_test::App, helper: EscrowHelper) {
    let balance = router_ref.wrap().query_balance("user3", "uluna").unwrap();
    let res = helper.arb_unbond(router_ref, "user3", 50_000000, Some(true)).unwrap();
    let new_balance = router_ref.wrap().query_balance("user3", "uluna").unwrap();

    res.assert_attribute("wasm", attr("burnt_amount", "50000000")).unwrap();
    res.assert_attribute("wasm", attr("withdraw_amount", "50030000")).unwrap();
    res.assert_attribute("wasm", attr("receive_amount", "48028800")).unwrap();
    res.assert_attribute("wasm", attr("protocol_fee", "500300")).unwrap();
    res.assert_attribute("wasm", attr("pool_fee", "1500900")).unwrap();
    assert_eq!(balance.amount + uint(48028800), new_balance.amount);
}

#[test]
fn provide_liquidity_and_arb_submit_twice() -> StdResult<()> {
    let mut router = mock_app();
    let router_ref = &mut router;
    let helper = EscrowHelper::init(router_ref, false);
    helper.arb_remove_eris_lsd(router_ref).unwrap();
    helper.arb_stader_insert(router_ref).unwrap();

    helper.stader_bond(router_ref, "user1", 100_000000, "uluna").unwrap();
    helper.stader_donate(router_ref, "user1", 10_000000, "uluna").unwrap();

    helper.arb_stader_fake_fill_arb_contract(router_ref);
    helper.stader_harvest(router_ref).unwrap();

    helper.arb_deposit(router_ref, "user1", 100_000000).unwrap();
    helper.arb_deposit(router_ref, "user2", 50_000000).unwrap();
    helper.arb_deposit(router_ref, "user3", 150_000000).unwrap();

    let user = helper.arb_query_user_info(router_ref, "user1").unwrap();
    assert_eq!(
        user,
        UserInfoResponse {
            utoken_amount: uint(100_000000),
            lp_amount: uint(100_000000)
        }
    );

    let amount = uint(10_000000u128);
    let profit_percent = dec("1.02");
    let eris_exchange_rate = dec("1.1");
    let fee_percent = dec("0.1");
    let absolute_profit = amount * profit_percent - amount - uint(1);
    let fee = absolute_profit * fee_percent;

    // EXECUTE ARB
    helper
        .arb_execute_whitelist(
            router_ref,
            ExecuteMsg::ExecuteArbitrage {
                msg: return_msg(&helper, amount, amount * profit_percent.div(eris_exchange_rate)),
                result_token: token_asset_info(helper.get_stader_token_addr()),
                wanted_profit: dec("0.01"),
            },
        )
        .unwrap();

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
                vault_available: uint(300_000000) - amount - fee,
                vault_takeable: uint(300_000000) - amount - fee,
                locked_user_withdrawls: uint(0),
                lsd_unbonding: uint(0),
                lsd_withdrawable: uint(0),
                lsd_xvalue: amount * profit_percent - uint(1),
                details: Some(vec![ClaimBalance {
                    name: "stader".to_string(),
                    withdrawable: uint(0),
                    unbonding: uint(0),
                    xbalance: uint(9272727),
                    xfactor: eris_exchange_rate
                }])
            },
            details: Some(eris::arb_vault::StateDetails {
                takeable_steps: vec![
                    (dec("0.010"), uint(139_890001)),
                    (dec("0.015"), uint(199_926001)),
                    (dec("0.020"), uint(259_962001)),
                    (dec("0.025"), uint(289_980001)),
                ]
            })
        }
    );

    // MOVE FUNDS TO UNBONDING
    helper
        .arb_execute_whitelist(
            router_ref,
            ExecuteMsg::UnbondFromLiquidStaking {
                names: Some(vec!["stader".to_string()]),
            },
        )
        .unwrap();

    // STATE MOVED TO UNBONDING
    let state = helper.arb_query_state(router_ref, Some(true)).unwrap();
    assert_eq!(
        state,
        StateResponse {
            exchange_rate: dec("1.0006"),
            total_lp_supply: uint(300_000000),
            balances: Balances {
                tvl_utoken: uint(300_000000) + absolute_profit - fee,
                vault_total: uint(300_000000) + absolute_profit - fee,
                vault_available: uint(300_000000) - amount - fee,
                vault_takeable: uint(300_000000) - amount - fee,
                locked_user_withdrawls: uint(0),
                // moved to unbonding
                lsd_unbonding: amount * profit_percent - uint(1),
                lsd_withdrawable: uint(0),
                lsd_xvalue: uint(0),
                details: Some(vec![ClaimBalance {
                    name: "stader".to_string(),
                    withdrawable: uint(0),
                    unbonding: uint(10199999),
                    xbalance: uint(0),
                    xfactor: eris_exchange_rate
                }])
            },
            details: Some(eris::arb_vault::StateDetails {
                takeable_steps: vec![
                    (dec("0.010"), uint(139_890001)),
                    (dec("0.015"), uint(199_926001)),
                    (dec("0.020"), uint(259_962001)),
                    (dec("0.025"), uint(289_980001)),
                ]
            })
        }
    );

    // WAIT SOME DAYS BEFORE START TO UNBONDING
    router_ref.next_block(DAY * 3);

    // SUBMIT BATCH
    helper.stader_submit_batch(router_ref).unwrap();
    router_ref.next_block(DAY * 3);

    // STATE IS STILL THE SAME AS in provide_liquidity_and_arb
    let state = helper.arb_query_state(router_ref, Some(true)).unwrap();
    assert_eq!(
        state,
        StateResponse {
            // rounding when unbonding, some is kept in the amplifier
            exchange_rate: dec("1.0006"),
            total_lp_supply: uint(300_000000),
            balances: Balances {
                tvl_utoken: uint(300_000000) + absolute_profit - fee,
                vault_total: uint(300_000000) + absolute_profit - fee,
                vault_available: uint(300_000000) - amount - fee,
                vault_takeable: uint(300_000000) - amount - fee,
                locked_user_withdrawls: uint(0),
                lsd_unbonding: amount * profit_percent - uint(1),
                lsd_withdrawable: uint(0),
                lsd_xvalue: uint(0),
                details: Some(vec![ClaimBalance {
                    name: "stader".to_string(),
                    withdrawable: uint(0),
                    unbonding: uint(10200000) - uint(1),
                    xbalance: uint(0),
                    // rounding increased exchange rate in the amplifier
                    xfactor: dec("1.1")
                }])
            },
            details: Some(eris::arb_vault::StateDetails {
                takeable_steps: vec![
                    (dec("0.010"), uint(139_890001)),
                    (dec("0.015"), uint(199_926001)),
                    (dec("0.020"), uint(259_962001)),
                    (dec("0.025"), uint(289_980001)),
                ]
            })
        }
    );

    // EXECUTE ARB 2
    helper
        .arb_execute_whitelist(
            router_ref,
            ExecuteMsg::ExecuteArbitrage {
                msg: return_msg(&helper, amount, amount * profit_percent.div(eris_exchange_rate)),
                result_token: token_asset_info(helper.get_stader_token_addr()),
                wanted_profit: dec("0.01"),
            },
        )
        .unwrap();

    // MOVE FUNDS TO UNBONDING
    helper
        .arb_execute_whitelist(
            router_ref,
            ExecuteMsg::UnbondFromLiquidStaking {
                names: Some(vec!["stader".to_string()]),
            },
        )
        .unwrap();

    // STATE MOVED TO UNBONDING
    let state = helper.arb_query_state(router_ref, Some(true)).unwrap();
    assert_eq!(
        state,
        StateResponse {
            // this does include staking rewards
            exchange_rate: dec("1.0012"),
            total_lp_supply: uint(300_000000),
            balances: Balances {
                tvl_utoken: uint(300_000000) + (absolute_profit - fee) * uint(2),
                vault_total: uint(300_000000) + (absolute_profit - fee) * uint(2),
                vault_available: uint(300_000000) - (amount + fee) * uint(2),
                vault_takeable: uint(300_000000) - (amount + fee) * uint(2),
                locked_user_withdrawls: uint(0),
                lsd_unbonding: amount * profit_percent * uint(2) - uint(2),
                lsd_withdrawable: uint(0),
                lsd_xvalue: uint(0),
                details: Some(vec![ClaimBalance {
                    name: "stader".to_string(),
                    withdrawable: uint(0),
                    unbonding: uint(10199999) * uint(2),
                    xbalance: uint(0),
                    xfactor: dec("1.100000000069370619")
                }])
            },
            details: Some(eris::arb_vault::StateDetails {
                takeable_steps: vec![
                    (dec("0.010"), uint(129780002)),
                    (dec("0.015"), uint(189852002)),
                    (dec("0.020"), uint(249924002)),
                    (dec("0.025"), uint(279960002)),
                ]
            })
        }
    );
    // WAIT SOME DAYS BEFORE START TO UNBONDING
    router_ref.next_block(DAY * 3);
    // SUBMIT BATCH
    helper.stader_submit_batch(router_ref).unwrap();
    router_ref.next_block(DAY * 3);
    let state = helper.arb_query_state(router_ref, Some(true)).unwrap();
    assert_eq!(
        state,
        StateResponse {
            exchange_rate: dec("1.0012"),
            total_lp_supply: uint(300_000000),
            balances: Balances {
                tvl_utoken: uint(300_000000) + (absolute_profit - fee) * uint(2),
                vault_total: uint(300_000000) + (absolute_profit - fee) * uint(2),
                vault_available: uint(300_000000) - (amount + fee) * uint(2),
                vault_takeable: uint(300_000000) - (amount + fee) * uint(2),
                locked_user_withdrawls: uint(0),
                // moved to unbonding
                lsd_unbonding: amount * profit_percent * uint(2) - uint(2),
                lsd_withdrawable: uint(0),
                lsd_xvalue: uint(0),
                details: Some(vec![ClaimBalance {
                    name: "stader".to_string(),
                    withdrawable: uint(0),
                    unbonding: uint(10199999) * uint(2),
                    xbalance: uint(0),
                    xfactor: dec("1.100000000069370619")
                }])
            },
            details: Some(eris::arb_vault::StateDetails {
                takeable_steps: vec![
                    (dec("0.010"), uint(129780002)),
                    (dec("0.015"), uint(189852002)),
                    (dec("0.020"), uint(249924002)),
                    (dec("0.025"), uint(279960002)),
                ]
            })
        }
    );

    router_ref.next_block(DAY * 13);
    helper.stader_reconcile(router_ref, 10200000).unwrap();

    // STATE IS STILL THE SAME AS in provide_liquidity_and_arb only withdrawable has changed
    let state = helper.arb_query_state(router_ref, Some(true)).unwrap();
    assert_eq!(
        state,
        StateResponse {
            exchange_rate: dec("1.0012"),
            total_lp_supply: uint(300_000000),
            balances: Balances {
                tvl_utoken: uint(300_000000) + (absolute_profit - fee) * uint(2),
                vault_total: uint(300_000000) + (absolute_profit - fee) * uint(2),
                vault_available: uint(300_000000) - (amount + fee) * uint(2),
                vault_takeable: uint(300_000000) - (amount + fee) * uint(2),
                locked_user_withdrawls: uint(0),
                lsd_unbonding: amount * profit_percent - uint(1),
                lsd_withdrawable: amount * profit_percent - uint(1),
                lsd_xvalue: uint(0),
                details: Some(vec![ClaimBalance {
                    name: "stader".to_string(),
                    withdrawable: uint(10200000) - uint(1),
                    unbonding: uint(10200000) - uint(1),
                    xbalance: uint(0),
                    xfactor: dec("1.100000000069370619")
                }])
            },
            details: Some(eris::arb_vault::StateDetails {
                takeable_steps: vec![
                    (dec("0.010"), uint(129780002)),
                    (dec("0.015"), uint(189852002)),
                    (dec("0.020"), uint(249924002)),
                    (dec("0.025"), uint(279960002)),
                ]
            })
        }
    );

    helper
        .arb_execute_whitelist(
            router_ref,
            ExecuteMsg::WithdrawFromLiquidStaking {
                names: None,
            },
        )
        .unwrap();

    // STATE IS STILL THE SAME AS in provide_liquidity_and_arb this time everything is withdrawn from the lsd vaults
    let state = helper.arb_query_state(router_ref, Some(true)).unwrap();
    assert_eq!(
        state,
        StateResponse {
            // because reconcile received more uluna than expected -> rounding correct again
            exchange_rate: dec("1.0012"),
            total_lp_supply: uint(300_000000),
            balances: Balances {
                tvl_utoken: uint(300_000000) + (absolute_profit - fee) * uint(2),
                vault_total: uint(300_000000) + (absolute_profit - fee) * uint(2),
                vault_available: uint(300_000000) - amount - fee + absolute_profit - fee,
                vault_takeable: uint(300_000000) - amount - fee + absolute_profit - fee,
                locked_user_withdrawls: uint(0),
                lsd_unbonding: amount * profit_percent - uint(1),
                lsd_withdrawable: uint(0),
                lsd_xvalue: uint(0),
                details: Some(vec![ClaimBalance {
                    name: "stader".to_string(),
                    withdrawable: uint(0),
                    unbonding: amount * profit_percent - uint(1),
                    xbalance: uint(0),
                    xfactor: dec("1.100000000069370619")
                }])
            },
            details: Some(eris::arb_vault::StateDetails {
                takeable_steps: vec![
                    (dec("0.010"), uint(139980001)),
                    (dec("0.015"), uint(200052001)),
                    (dec("0.020"), uint(260124001)),
                    (dec("0.025"), uint(290160001)),
                ]
            })
        }
    );

    check_normal_withdraw_2(router_ref, &helper);
    check_immediate_withdraw_2(router_ref, helper);

    Ok(())
}

fn check_normal_withdraw_2(router_ref: &mut cw_multi_test::App, helper: &EscrowHelper) {
    let balance = router_ref.wrap().query_balance("user2", "uluna").unwrap();
    let res = helper.arb_unbond(router_ref, "user2", 50_000000, None).unwrap();
    let balance2 = router_ref.wrap().query_balance("user2", "uluna").unwrap();

    res.assert_attribute("wasm", attr("burnt_amount", "50000000")).unwrap();
    res.assert_attribute("wasm", attr("withdraw_amount", "50060000")).unwrap();
    res.assert_attribute("wasm", attr("receive_amount", "49559400")).unwrap();
    res.assert_attribute("wasm", attr("protocol_fee", "500600")).unwrap();
    assert_eq!(balance.amount, balance2.amount);

    router_ref.next_block(DAY * 25);

    let balance3 = router_ref.wrap().query_balance("user2", "uluna").unwrap();
    assert_eq!(balance3.amount, balance2.amount);
    helper.arb_withdraw(router_ref, "user2").unwrap();
    let balance4 = router_ref.wrap().query_balance("user2", "uluna").unwrap();
    assert_eq!(balance3.amount + uint(49559400), balance4.amount);
}

fn check_immediate_withdraw_2(router_ref: &mut cw_multi_test::App, helper: EscrowHelper) {
    let balance = router_ref.wrap().query_balance("user3", "uluna").unwrap();
    let res = helper.arb_unbond(router_ref, "user3", 50_000000, Some(true)).unwrap();
    let new_balance = router_ref.wrap().query_balance("user3", "uluna").unwrap();

    res.assert_attribute("wasm", attr("burnt_amount", "50000000")).unwrap();
    res.assert_attribute("wasm", attr("withdraw_amount", "50060000")).unwrap();
    res.assert_attribute("wasm", attr("receive_amount", "48057600")).unwrap();
    res.assert_attribute("wasm", attr("protocol_fee", "500600")).unwrap();
    res.assert_attribute("wasm", attr("pool_fee", "1501800")).unwrap();
    assert_eq!(balance.amount + uint(48057600), new_balance.amount);
}

fn return_msg(
    helper: &EscrowHelper,
    amount: Uint128,
    return_amount: Uint128,
) -> eris::arb_vault::ExecuteSubMsg {
    eris::arb_vault::ExecuteSubMsg {
        contract_addr: Some(helper.base.arb_fake_contract.get_address_string()),
        msg: to_binary(&eris_tests::arb_contract::ExecuteMsg::ReturnAsset {
            asset: token_asset(helper.get_stader_token_addr(), return_amount),
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
