use cosmwasm_std::testing::mock_info;
use eris::adapters::asset::AssetEx;
use eris::adapters::farm::Farm;
use eris::adapters::hub::Hub;

use crate::contract::execute;
use crate::testing::helpers::mock_env_at_timestamp_height;

use super::helpers::{mock_env_at_timestamp, setup_test};
use std::vec;

use astroport::asset::{native_asset, native_asset_info, token_asset, token_asset_info};
use cosmwasm_std::testing::MOCK_CONTRACT_ADDR;
use cosmwasm_std::{coins, Addr, Uint128};

use eris::ampz::{CallbackMsg, CallbackWrapper, ExecuteMsg};

use crate::constants::CONTRACT_DENOM;

fn astro() -> Addr {
    Addr::unchecked("astro")
}

#[test]
fn check_callback_deposit_amplifier_receiver() {
    let mut deps = setup_test();
    let receiver = Addr::unchecked("other_wallet");

    // any executor
    deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, coins(100, CONTRACT_DENOM));
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::Callback(CallbackWrapper {
            id: 1,
            user: Addr::unchecked("user"),
            message: CallbackMsg::FinishExecution {
                destination: eris::ampz::DestinationRuntime::DepositAmplifier {
                    receiver: Some(receiver.clone()),
                },
                source: eris::ampz::Source::Claim,
                executor: Addr::unchecked("executor"),
            },
        }),
    )
    .unwrap();
    assert_eq!(res.messages.len(), 3);
    assert_eq!(
        res.messages[0].msg,
        native_asset(CONTRACT_DENOM.into(), Uint128::new(1))
            .transfer_msg(&Addr::unchecked("fee_receiver"))
            .unwrap()
    );
    assert_eq!(
        res.messages[1].msg,
        native_asset(CONTRACT_DENOM.into(), Uint128::new(2))
            .transfer_msg(&Addr::unchecked("executor"))
            .unwrap()
    );
    assert_eq!(
        res.messages[2].msg,
        // very important that the user receives the bond result
        Hub(Addr::unchecked("hub"))
            .bond_msg(CONTRACT_DENOM, 97, Some(receiver.to_string()))
            .unwrap(),
    );

    // user as executor "manual execution"
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::Callback(CallbackWrapper {
            id: 1,
            user: Addr::unchecked("user"),
            message: CallbackMsg::FinishExecution {
                destination: eris::ampz::DestinationRuntime::DepositAmplifier {
                    receiver: Some(receiver.clone()),
                },
                source: eris::ampz::Source::Claim,
                executor: Addr::unchecked("user"),
            },
        }),
    )
    .unwrap();

    assert_eq!(res.messages.len(), 2);
    assert_eq!(
        res.messages[0].msg,
        native_asset(CONTRACT_DENOM.into(), Uint128::new(1))
            .transfer_msg(&Addr::unchecked("fee_receiver"))
            .unwrap()
    );
    assert_eq!(
        res.messages[1].msg,
        // very important that the user receives the bond result
        Hub(Addr::unchecked("hub"))
            .bond_msg(CONTRACT_DENOM, 99, Some(receiver.to_string()))
            .unwrap(),
    );

    // protocol controller as executor
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::Callback(CallbackWrapper {
            id: 1,
            user: Addr::unchecked("user"),
            message: CallbackMsg::FinishExecution {
                destination: eris::ampz::DestinationRuntime::DepositAmplifier {
                    receiver: Some(receiver.clone()),
                },
                source: eris::ampz::Source::Claim,
                executor: Addr::unchecked("controller"),
            },
        }),
    )
    .unwrap();

    assert_eq!(res.messages.len(), 2);
    assert_eq!(
        res.messages[0].msg,
        native_asset(CONTRACT_DENOM.into(), Uint128::new(3))
            .transfer_msg(&Addr::unchecked("fee_receiver"))
            .unwrap()
    );
    assert_eq!(
        res.messages[1].msg,
        // very important that the user receives the bond result
        Hub(Addr::unchecked("hub"))
            .bond_msg(CONTRACT_DENOM, 97, Some(receiver.to_string()))
            .unwrap(),
    );
}

#[test]
fn check_callback_deposit_farm_receiver() {
    let mut deps = setup_test();
    let receiver = Addr::unchecked("other_wallet");

    deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, coins(100, CONTRACT_DENOM));
    deps.querier.set_cw20_balance(MOCK_CONTRACT_ADDR, astro().as_str(), 1000);

    // any executor
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp_height(1000, 300),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::Callback(CallbackWrapper {
            id: 1,
            user: Addr::unchecked("user"),
            message: CallbackMsg::FinishExecution {
                destination: eris::ampz::DestinationRuntime::DepositFarm {
                    receiver: Some(receiver.clone()),
                    asset_infos: vec![
                        native_asset_info(CONTRACT_DENOM.into()),
                        token_asset_info(astro()),
                    ],
                    farm: "farm".to_string(),
                },
                source: eris::ampz::Source::Claim,
                executor: Addr::unchecked("executor"),
            },
        }),
    )
    .unwrap();
    assert_eq!(res.messages.len(), 6);
    assert_eq!(
        res.messages[0].msg,
        native_asset(CONTRACT_DENOM.into(), Uint128::new(1))
            .transfer_msg(&Addr::unchecked("fee_receiver"))
            .unwrap()
    );
    assert_eq!(
        res.messages[1].msg,
        native_asset(CONTRACT_DENOM.into(), Uint128::new(2))
            .transfer_msg(&Addr::unchecked("executor"))
            .unwrap()
    );
    assert_eq!(
        res.messages[2].msg,
        token_asset(astro(), Uint128::new(10))
            .transfer_msg(&Addr::unchecked("fee_receiver"))
            .unwrap()
    );
    assert_eq!(
        res.messages[3].msg,
        token_asset(astro(), Uint128::new(20)).transfer_msg(&Addr::unchecked("executor")).unwrap()
    );
    assert_eq!(
        res.messages[4].msg,
        token_asset(astro(), Uint128::new(970))
            .increase_allowance_msg("farm".into(), Some(cw20::Expiration::AtHeight(300 + 1)))
            .unwrap(),
    );
    assert_eq!(
        res.messages[5].msg,
        // very important that the user receives the bond result
        Farm(Addr::unchecked("farm"))
            .bond_assets_msg(
                vec![
                    native_asset(CONTRACT_DENOM.into(), Uint128::new(97)),
                    token_asset(astro(), Uint128::new(970))
                ],
                coins(97, CONTRACT_DENOM),
                Some(receiver.into())
            )
            .unwrap(),
    );
}

#[test]
fn check_callback_swap_to() {
    let mut deps = setup_test();
    let receiver = Addr::unchecked("other_wallet");

    // any executor
    deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, coins(100, "ibc/xxx"));
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::Callback(CallbackWrapper {
            id: 1,
            user: Addr::unchecked("user"),
            message: CallbackMsg::FinishExecution {
                destination: eris::ampz::DestinationRuntime::SendSwapResultToUser {
                    receiver: Some(receiver.clone()),
                    asset_info: native_asset_info("ibc/xxx".to_string()),
                },
                source: eris::ampz::Source::Claim,
                executor: Addr::unchecked("executor"),
            },
        }),
    )
    .unwrap();
    assert_eq!(res.messages.len(), 3);
    assert_eq!(
        res.messages[0].msg,
        native_asset("ibc/xxx".into(), Uint128::new(1))
            .transfer_msg(&Addr::unchecked("fee_receiver"))
            .unwrap()
    );
    assert_eq!(
        res.messages[1].msg,
        native_asset("ibc/xxx".into(), Uint128::new(2))
            .transfer_msg(&Addr::unchecked("executor"))
            .unwrap()
    );
    assert_eq!(
        res.messages[2].msg,
        native_asset("ibc/xxx".into(), Uint128::new(97)).transfer_msg(&receiver).unwrap(),
    );
}
