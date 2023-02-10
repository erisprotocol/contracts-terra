use cosmwasm_std::testing::mock_info;
use eris::adapters::ampz::Ampz;
use eris::adapters::asset::AssetEx;
use eris::adapters::compounder::Compounder;
use eris::adapters::farm::Farm;
use eris::adapters::hub::Hub;

use crate::protos::msgex::CosmosMsgEx;
use crate::testing::helpers::mock_env_at_timestamp_height;
use crate::{contract::execute, error::ContractError};

use super::helpers::{mock_env_at_timestamp, setup_test};
use std::vec;

use astroport::asset::{native_asset, native_asset_info, token_asset, token_asset_info};
use cosmwasm_std::testing::MOCK_CONTRACT_ADDR;
use cosmwasm_std::{coin, coins, Addr, Uint128};

use eris::ampz::{CallbackMsg, CallbackWrapper, ExecuteMsg};

use crate::constants::CONTRACT_DENOM;

fn astro() -> Addr {
    Addr::unchecked("astro")
}

#[test]
fn check_callback_authz_deposit() {
    let mut deps = setup_test();

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("user", &[]),
        ExecuteMsg::Callback(CallbackWrapper {
            id: 1,
            user: Addr::unchecked("user"),
            message: CallbackMsg::AuthzDeposit {
                user_balance_start: vec![],
                max_amount: None,
            },
        }),
    )
    .unwrap_err();

    assert_eq!(res, ContractError::CallbackOnlyCalledByContract {});

    // check empty
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::Callback(CallbackWrapper {
            id: 1,
            user: Addr::unchecked("user"),
            message: CallbackMsg::AuthzDeposit {
                user_balance_start: vec![],
                max_amount: None,
            },
        }),
    )
    .unwrap_err();
    assert_eq!(res, ContractError::NothingToDeposit {});

    // check coin
    deps.querier.bank_querier.update_balance("user", coins(100, CONTRACT_DENOM));
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::Callback(CallbackWrapper {
            id: 1,
            user: Addr::unchecked("user"),
            message: CallbackMsg::AuthzDeposit {
                user_balance_start: vec![native_asset(CONTRACT_DENOM.into(), Uint128::new(100))],
                max_amount: None,
            },
        }),
    )
    .unwrap_err();
    assert_eq!(res, ContractError::NothingToDeposit {});

    // check cw20
    deps.querier.set_cw20_balance("user", astro().as_str(), 150);
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::Callback(CallbackWrapper {
            id: 1,
            user: Addr::unchecked("user"),
            message: CallbackMsg::AuthzDeposit {
                user_balance_start: vec![token_asset(astro(), Uint128::new(150))],
                max_amount: None,
            },
        }),
    )
    .unwrap_err();
    assert_eq!(res, ContractError::NothingToDeposit {});

    deps.querier.set_cw20_balance("user", "nothing", 0);
    // both
    let env = mock_env_at_timestamp_height(1000, 300);
    let res = execute(
        deps.as_mut(),
        env.clone(),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::Callback(CallbackWrapper {
            id: 1,
            user: Addr::unchecked("user"),
            message: CallbackMsg::AuthzDeposit {
                user_balance_start: vec![
                    native_asset(CONTRACT_DENOM.into(), Uint128::new(70)),
                    token_asset(astro(), Uint128::new(100)),
                    token_asset(Addr::unchecked("nothing"), Uint128::new(0)),
                ],
                max_amount: None,
            },
        }),
    )
    .unwrap();

    assert_eq!(res.messages.len(), 2);

    assert_eq!(
        res.messages[0].msg,
        token_asset(astro(), Uint128::new(50))
            .increase_allowance_msg(
                MOCK_CONTRACT_ADDR.into(),
                Some(cw20::Expiration::AtHeight(env.block.height + 1))
            )
            .unwrap()
            .to_authz_msg("user", &env)
            .unwrap()
    );
    assert_eq!(
        res.messages[1].msg,
        Ampz(Addr::unchecked(MOCK_CONTRACT_ADDR))
            .deposit(
                vec![
                    // only the delta is being deposited
                    native_asset(CONTRACT_DENOM.into(), Uint128::new(30)),
                    token_asset(astro(), Uint128::new(50))
                ],
                vec![coin(30, CONTRACT_DENOM)]
            )
            .unwrap()
            .to_authz_msg("user", &env)
            .unwrap(),
    );
}

#[test]
fn check_callback_swap() {
    let mut deps = setup_test();

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("user", &[]),
        ExecuteMsg::Callback(CallbackWrapper {
            id: 1,
            user: Addr::unchecked("user"),
            message: CallbackMsg::Swap {
                asset_infos: vec![
                    native_asset_info(CONTRACT_DENOM.into()),
                    native_asset_info("native2".into()),
                    token_asset_info(astro()),
                    token_asset_info(Addr::unchecked("token2")),
                ],
                into: native_asset_info(CONTRACT_DENOM.into()),
            },
        }),
    )
    .unwrap_err();

    assert_eq!(res, ContractError::CallbackOnlyCalledByContract {});

    deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, coins(100, CONTRACT_DENOM));
    deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, coins(0, "nothing"));
    deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, coins(1, "small"));
    deps.querier.set_cw20_balance(MOCK_CONTRACT_ADDR, "astro", 10);
    deps.querier.set_cw20_balance(MOCK_CONTRACT_ADDR, "some", 20);

    // both
    let env = mock_env_at_timestamp_height(1000, 300);
    let res = execute(
        deps.as_mut(),
        env.clone(),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::Callback(CallbackWrapper {
            id: 1,
            user: Addr::unchecked("user"),
            message: CallbackMsg::Swap {
                asset_infos: vec![
                    native_asset_info(CONTRACT_DENOM.into()),
                    native_asset_info("nothing".into()),
                    native_asset_info("small".into()),
                    token_asset_info(astro()),
                    token_asset_info(Addr::unchecked("some")),
                ],
                into: native_asset_info(CONTRACT_DENOM.into()),
            },
        }),
    )
    .unwrap();

    assert_eq!(res.messages.len(), 3);

    assert_eq!(
        res.messages[0].msg,
        token_asset(astro(), Uint128::new(10))
            .increase_allowance_msg(
                "zapper".into(),
                Some(cw20::Expiration::AtHeight(env.block.height + 1))
            )
            .unwrap()
    );
    assert_eq!(
        res.messages[1].msg,
        token_asset(Addr::unchecked("some"), Uint128::new(20))
            .increase_allowance_msg(
                "zapper".into(),
                Some(cw20::Expiration::AtHeight(env.block.height + 1))
            )
            .unwrap()
    );
    assert_eq!(
        res.messages[2].msg,
        Compounder(Addr::unchecked("zapper"))
            .multi_swap_msg(
                vec![
                    native_asset("small".into(), Uint128::new(1)),
                    token_asset(astro(), Uint128::new(10)),
                    token_asset(Addr::unchecked("some"), Uint128::new(20)),
                ],
                native_asset_info(CONTRACT_DENOM.into()),
                coins(1, "small"),
                None
            )
            .unwrap(),
    );
}

#[test]
fn check_callback_deposit_amplifier() {
    let mut deps = setup_test();

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("user", &[]),
        ExecuteMsg::Callback(CallbackWrapper {
            id: 1,
            user: Addr::unchecked("user"),
            message: CallbackMsg::FinishExecution {
                destination: eris::ampz::DestinationRuntime::DepositAmplifier {},
                executor: Addr::unchecked("executor"),
            },
        }),
    )
    .unwrap_err();

    assert_eq!(res, ContractError::CallbackOnlyCalledByContract {});

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
                destination: eris::ampz::DestinationRuntime::DepositAmplifier {},
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
        Hub(Addr::unchecked("hub")).bond_msg(CONTRACT_DENOM, 97, Some("user".into())).unwrap(),
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
                destination: eris::ampz::DestinationRuntime::DepositAmplifier {},
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
        Hub(Addr::unchecked("hub")).bond_msg(CONTRACT_DENOM, 99, Some("user".into())).unwrap(),
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
                destination: eris::ampz::DestinationRuntime::DepositAmplifier {},
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
        Hub(Addr::unchecked("hub")).bond_msg(CONTRACT_DENOM, 97, Some("user".into())).unwrap(),
    );
}

#[test]
fn check_callback_deposit_farm() {
    let mut deps = setup_test();

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("user", &[]),
        ExecuteMsg::Callback(CallbackWrapper {
            id: 1,
            user: Addr::unchecked("user"),
            message: CallbackMsg::FinishExecution {
                destination: eris::ampz::DestinationRuntime::DepositFarm {
                    asset_infos: vec![],
                    farm: "farm".to_string(),
                },
                executor: Addr::unchecked("executor"),
            },
        }),
    )
    .unwrap_err();

    assert_eq!(res, ContractError::CallbackOnlyCalledByContract {});

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
                    asset_infos: vec![
                        native_asset_info(CONTRACT_DENOM.into()),
                        token_asset_info(astro()),
                    ],
                    farm: "farm".to_string(),
                },
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
                Some("user".into())
            )
            .unwrap(),
    );

    // user as executor "manual execution"
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp_height(1000, 300),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::Callback(CallbackWrapper {
            id: 1,
            user: Addr::unchecked("user"),
            message: CallbackMsg::FinishExecution {
                destination: eris::ampz::DestinationRuntime::DepositFarm {
                    asset_infos: vec![
                        native_asset_info(CONTRACT_DENOM.into()),
                        token_asset_info(astro()),
                    ],
                    farm: "farm".to_string(),
                },
                executor: Addr::unchecked("user"),
            },
        }),
    )
    .unwrap();

    assert_eq!(res.messages.len(), 4);
    assert_eq!(
        res.messages[0].msg,
        native_asset(CONTRACT_DENOM.into(), Uint128::new(1))
            .transfer_msg(&Addr::unchecked("fee_receiver"))
            .unwrap()
    );
    assert_eq!(
        res.messages[1].msg,
        token_asset(astro(), Uint128::new(10))
            .transfer_msg(&Addr::unchecked("fee_receiver"))
            .unwrap()
    );
    assert_eq!(
        res.messages[2].msg,
        token_asset(astro(), Uint128::new(990))
            .increase_allowance_msg("farm".into(), Some(cw20::Expiration::AtHeight(300 + 1)))
            .unwrap(),
    );
    assert_eq!(
        res.messages[3].msg,
        // very important that the user receives the bond result
        Farm(Addr::unchecked("farm"))
            .bond_assets_msg(
                vec![
                    native_asset(CONTRACT_DENOM.into(), Uint128::new(99)),
                    token_asset(astro(), Uint128::new(990))
                ],
                coins(99, CONTRACT_DENOM),
                Some("user".into())
            )
            .unwrap(),
    );

    // protocol controller as executor
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp_height(1000, 300),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::Callback(CallbackWrapper {
            id: 1,
            user: Addr::unchecked("user"),
            message: CallbackMsg::FinishExecution {
                destination: eris::ampz::DestinationRuntime::DepositFarm {
                    asset_infos: vec![
                        native_asset_info(CONTRACT_DENOM.into()),
                        token_asset_info(astro()),
                    ],
                    farm: "farm".to_string(),
                },
                executor: Addr::unchecked("controller"),
            },
        }),
    )
    .unwrap();

    assert_eq!(res.messages.len(), 4);
    assert_eq!(
        res.messages[0].msg,
        native_asset(CONTRACT_DENOM.into(), Uint128::new(3))
            .transfer_msg(&Addr::unchecked("fee_receiver"))
            .unwrap()
    );
    assert_eq!(
        res.messages[1].msg,
        token_asset(astro(), Uint128::new(30))
            .transfer_msg(&Addr::unchecked("fee_receiver"))
            .unwrap()
    );
    assert_eq!(
        res.messages[2].msg,
        token_asset(astro(), Uint128::new(970))
            .increase_allowance_msg("farm".into(), Some(cw20::Expiration::AtHeight(300 + 1)))
            .unwrap(),
    );
    assert_eq!(
        res.messages[3].msg,
        // very important that the user receives the bond result
        Farm(Addr::unchecked("farm"))
            .bond_assets_msg(
                vec![
                    native_asset(CONTRACT_DENOM.into(), Uint128::new(97)),
                    token_asset(astro(), Uint128::new(970))
                ],
                coins(97, CONTRACT_DENOM),
                Some("user".into())
            )
            .unwrap(),
    );
}
