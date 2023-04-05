use astroport::asset::{token_asset, token_asset_info};
use cosmwasm_std::{coin, to_binary, Addr, Decimal, StdResult, Uint128};
use eris_tests::mock_app;
use eris_tests::{gov_helper::EscrowHelper, TerraAppExtension};
use std::str::FromStr;
use std::vec;

use eris::{
    arb_vault::{ExecuteMsg, LsdConfig, UtilizationMethod},
    constants::DAY,
};

#[test]
fn update_config_utilization() -> StdResult<()> {
    let mut router = mock_app();
    let router_ref = &mut router;
    let helper = EscrowHelper::init(router_ref, false);

    let config = helper.arb_query_config(router_ref).unwrap();

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
            router_ref,
            ExecuteMsg::UpdateConfig {
                utilization_method: Some(UtilizationMethod::Steps(vec![(dec("0.1"), dec("1"))])),
                unbond_time_s: None,
                disable_lsd: None,
                insert_lsd: None,
                remove_lsd: None,
                force_remove_lsd: None,
                fee_config: None,
                set_whitelist: None,
                remove_whitelist: None,
            },
            "user",
        )
        .unwrap_err();
    assert_eq!("Unauthorized", result.root_cause().to_string());

    let result = helper
        .arb_execute(
            router_ref,
            ExecuteMsg::UpdateConfig {
                utilization_method: Some(UtilizationMethod::Steps(vec![(
                    dec("0.1"),
                    dec("1.00001"),
                )])),
                unbond_time_s: None,
                disable_lsd: None,
                insert_lsd: None,
                remove_lsd: None,
                force_remove_lsd: None,
                fee_config: None,
                set_whitelist: None,
                remove_whitelist: None,
            },
        )
        .unwrap_err();
    assert_eq!("Specified step max take is too high", result.root_cause().to_string());

    helper
        .arb_execute(
            router_ref,
            ExecuteMsg::UpdateConfig {
                utilization_method: Some(UtilizationMethod::Steps(vec![(dec("0.1"), dec("1"))])),
                unbond_time_s: None,
                disable_lsd: None,
                insert_lsd: None,
                remove_lsd: None,
                force_remove_lsd: None,
                fee_config: None,
                set_whitelist: None,
                remove_whitelist: None,
            },
        )
        .unwrap();

    let config = helper.arb_query_config(router_ref).unwrap();

    assert_eq!(
        config.config.utilization_method,
        UtilizationMethod::Steps(vec![(dec("0.1"), dec("1"))])
    );

    Ok(())
}

#[test]
fn update_config_disable() -> StdResult<()> {
    let mut router = mock_app();
    let router_ref = &mut router;
    let helper = EscrowHelper::init(router_ref, false);

    let amount = Uint128::new(10_000000u128);
    let profit_percent = dec("1.02");

    helper.hub_bond(router_ref, "user1", 100_000000, "uluna").unwrap();
    helper.arb_fake_fill_arb_contract(router_ref);
    helper.arb_deposit(router_ref, "user1", 100_000000).unwrap();

    let result = helper
        .arb_execute(
            router_ref,
            ExecuteMsg::UpdateConfig {
                utilization_method: None,
                unbond_time_s: None,
                disable_lsd: Some("unknown".to_string()),
                insert_lsd: None,
                remove_lsd: None,
                force_remove_lsd: None,
                fee_config: None,
                set_whitelist: None,
                remove_whitelist: None,
            },
        )
        .unwrap_err();
    assert_eq!("Adapter not found: unknown", result.root_cause().to_string());

    helper
        .arb_execute(
            router_ref,
            ExecuteMsg::UpdateConfig {
                utilization_method: None,
                unbond_time_s: None,
                disable_lsd: Some("eris".to_string()),
                insert_lsd: None,
                remove_lsd: None,
                force_remove_lsd: None,
                fee_config: None,
                set_whitelist: None,
                remove_whitelist: None,
            },
        )
        .unwrap();

    let config = helper.arb_query_config(router_ref).unwrap();

    assert_eq!(
        config.config.lsds,
        vec![LsdConfig {
            disabled: true,
            name: "eris".to_string(),
            lsd_type: eris::arb_vault::LsdType::Eris {
                addr: helper.base.hub.get_address(),
                cw20: helper.get_ustake_addr()
            }
        }]
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

    assert_eq!(res.root_cause().to_string(), "Adapter eris is disabled");

    Ok(())
}

#[test]
fn update_config_remove() -> StdResult<()> {
    let mut router = mock_app();
    let router_ref = &mut router;
    let helper = EscrowHelper::init(router_ref, false);

    let amount = Uint128::new(10_000000u128);
    let profit_percent = dec("1.02");

    helper.hub_bond(router_ref, "user1", 100_000000, "uluna").unwrap();
    helper.arb_fake_fill_arb_contract(router_ref);
    helper.arb_deposit(router_ref, "user1", 100_000000).unwrap();

    // EXECUTE ARB
    helper
        .arb_execute_whitelist(
            router_ref,
            ExecuteMsg::ExecuteArbitrage {
                msg: return_msg(&helper, amount, amount * profit_percent),
                result_token: token_asset_info(helper.get_ustake_addr()),
                wanted_profit: dec("0.01"),
            },
        )
        .unwrap();
    assert_cant_remove(&helper, router_ref);

    // UNBOND
    helper
        .arb_execute_whitelist(
            router_ref,
            ExecuteMsg::UnbondFromLiquidStaking {
                names: None,
            },
        )
        .unwrap();
    assert_cant_remove(&helper, router_ref);

    // SUBMIT BATCH
    router_ref.next_block(DAY * 3);
    helper.hub_submit_batch(router_ref).unwrap();
    assert_cant_remove(&helper, router_ref);

    // RECONCILE
    router_ref.next_block(DAY * 21 + 1);
    helper.hub_reconcile(router_ref, 10200000).unwrap();
    assert_cant_remove(&helper, router_ref);

    // WITHDRAW
    helper
        .arb_execute_whitelist(
            router_ref,
            ExecuteMsg::WithdrawFromLiquidStaking {
                names: None,
            },
        )
        .unwrap();

    // now it can be removed
    helper
        .arb_execute(
            router_ref,
            ExecuteMsg::UpdateConfig {
                utilization_method: None,
                unbond_time_s: None,
                disable_lsd: None,
                insert_lsd: None,
                remove_lsd: Some("eris".to_string()),
                force_remove_lsd: None,
                fee_config: None,
                set_whitelist: None,
                remove_whitelist: None,
            },
        )
        .unwrap();

    let config = helper.arb_query_config(router_ref).unwrap();

    assert_eq!(config.config.lsds, vec![]);
    Ok(())
}

#[test]
fn update_config_insert() -> StdResult<()> {
    let mut router = mock_app();
    let router_ref = &mut router;
    let helper = EscrowHelper::init(router_ref, false);

    let res = helper
        .arb_execute(
            router_ref,
            ExecuteMsg::UpdateConfig {
                utilization_method: None,
                unbond_time_s: None,
                disable_lsd: None,
                insert_lsd: Some(LsdConfig {
                    disabled: false,
                    name: "eris".to_string(),
                    lsd_type: eris::arb_vault::LsdType::Eris {
                        addr: "xx".to_string(),
                        cw20: "yy".to_string(),
                    },
                }),
                remove_lsd: None,
                force_remove_lsd: None,
                fee_config: None,
                set_whitelist: None,
                remove_whitelist: None,
            },
        )
        .unwrap_err();

    assert_eq!(res.root_cause().to_string(), "Adapter duplicated: eris");

    // cant supply not working contract
    helper
        .arb_execute(
            router_ref,
            ExecuteMsg::UpdateConfig {
                utilization_method: None,
                unbond_time_s: None,
                disable_lsd: None,
                insert_lsd: Some(LsdConfig {
                    disabled: false,
                    name: "other".to_string(),
                    lsd_type: eris::arb_vault::LsdType::Eris {
                        addr: "xxx".to_string(),
                        cw20: "yyy".to_string(),
                    },
                }),
                remove_lsd: None,
                force_remove_lsd: None,
                fee_config: None,
                set_whitelist: None,
                remove_whitelist: None,
            },
        )
        .unwrap_err();

    helper
        .arb_execute(
            router_ref,
            ExecuteMsg::UpdateConfig {
                utilization_method: None,
                unbond_time_s: None,
                disable_lsd: None,
                insert_lsd: Some(LsdConfig {
                    disabled: false,
                    name: "other".to_string(),
                    lsd_type: eris::arb_vault::LsdType::Stader {
                        addr: helper.base.stader.get_address_string(),
                        cw20: helper.base.steak_token.get_address_string(),
                    },
                }),
                remove_lsd: None,
                force_remove_lsd: None,
                fee_config: None,
                set_whitelist: None,
                remove_whitelist: None,
            },
        )
        .unwrap();

    let config = helper.arb_query_config(router_ref).unwrap();

    assert_eq!(
        config.config.lsds,
        vec![
            LsdConfig {
                disabled: false,
                name: "eris".to_string(),
                lsd_type: eris::arb_vault::LsdType::Eris {
                    addr: helper.base.hub.get_address(),
                    cw20: helper.get_ustake_addr()
                }
            },
            LsdConfig {
                disabled: false,
                name: "other".to_string(),
                lsd_type: eris::arb_vault::LsdType::Stader {
                    addr: helper.base.stader.get_address(),
                    cw20: helper.base.steak_token.get_address()
                }
            }
        ]
    );
    Ok(())
}

fn assert_cant_remove(helper: &EscrowHelper, router_ref: &mut cw_multi_test::App) {
    let res = helper
        .arb_execute(
            router_ref,
            ExecuteMsg::UpdateConfig {
                utilization_method: None,
                unbond_time_s: None,
                disable_lsd: None,
                insert_lsd: None,
                remove_lsd: Some("eris".to_string()),
                force_remove_lsd: None,
                fee_config: None,
                set_whitelist: None,
                remove_whitelist: None,
            },
        )
        .unwrap_err();

    assert_eq!(res.root_cause().to_string(), "Cannot remove an adapter that has funds");
}

// fn uint(val: u128) -> Uint128 {
//     Uint128::new(val)
// }

fn dec(val: &str) -> Decimal {
    Decimal::from_str(val).unwrap()
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
