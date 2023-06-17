use cosmwasm_std::testing::{mock_env, mock_info};
use eris::adapters::asset::AssetEx;
use eris::adapters::generator::Generator;
use eris::constants::{DAY, HOUR};

use crate::adapters::capapult::{CapapultLocker, CapapultMarket};
use crate::protos::msgex::{CosmosMsgEx, CosmosMsgsEx};
use crate::testing::helpers::finish_amplifier;
use crate::{contract::execute, error::ContractError};

use super::helpers::{mock_env_at_timestamp, setup_test};
use std::vec;

use astroport::asset::{native_asset, native_asset_info, token_asset, token_asset_info};
use cosmwasm_std::testing::MOCK_CONTRACT_ADDR;
use cosmwasm_std::{coins, Addr, Uint128};

use eris::ampz::{CallbackMsg, ExecuteMsg, Execution, Schedule};
use protobuf::SpecialFields;

use crate::constants::CONTRACT_DENOM;
use crate::protos::authz::MsgExec;
use crate::protos::proto::MsgWithdrawDelegatorReward;

fn astro() -> Addr {
    Addr::unchecked("astro")
}

fn solid() -> Addr {
    Addr::unchecked("solid")
}

fn user() -> Addr {
    Addr::unchecked("user")
}
fn fee_receiver() -> Addr {
    Addr::unchecked("fee_receiver")
}

#[test]
fn check_execution_source_claim_deposit_amplifier() {
    let mut deps = setup_test();

    let interval_s = 6 * HOUR;
    let execution = Execution {
        destination: eris::ampz::DestinationState::DepositAmplifier {
            receiver: None,
        },
        schedule: Schedule {
            interval_s,
            start: None,
        },
        user: "user".into(),
        source: eris::ampz::Source::Claim,
    };

    let finish_execution = CallbackMsg::FinishExecution {
        destination: eris::ampz::DestinationRuntime::DepositAmplifier {
            receiver: None,
        },
        executor: Addr::unchecked("controller"),
    };

    execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: false,
            execution,
        },
    )
    .unwrap();

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("controller", &[]),
        ExecuteMsg::Execute {
            id: Uint128::new(1),
        },
    )
    .unwrap();

    // claim + deposit + finish
    assert_eq!(res.messages.len(), 3);
    assert_eq!(
        res.messages[0].msg,
        MsgExec {
            grantee: MOCK_CONTRACT_ADDR.to_string(),
            msgs: vec![MsgWithdrawDelegatorReward {
                delegator_address: "user".to_string(),
                validator_address: "val1".to_string(),
                special_fields: SpecialFields::default()
            }
            .to_any()
            .unwrap()],
            special_fields: SpecialFields::default()
        }
        .to_authz_cosmos_msg()
    );

    assert_eq!(
        res.messages[1].msg,
        CallbackMsg::AuthzDeposit {
            user_balance_start: vec![native_asset(CONTRACT_DENOM.to_string(), Uint128::new(0))],
            max_amount: None
        }
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &user())
        .unwrap()
    );

    assert_eq!(
        res.messages[2].msg,
        finish_execution.into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &user()).unwrap()
    );
}

#[test]
fn check_execution_source_wallet_native_deposit_amplifier() {
    let mut deps = setup_test();
    deps.querier.bank_querier.update_balance("user", coins(50, CONTRACT_DENOM));

    let interval_s = 6 * HOUR;
    let execution = Execution {
        destination: eris::ampz::DestinationState::DepositAmplifier {
            receiver: None,
        },
        schedule: Schedule {
            interval_s,
            start: None,
        },
        user: "user".into(),
        source: eris::ampz::Source::Wallet {
            over: native_asset(CONTRACT_DENOM.into(), Uint128::new(100)),
            max_amount: Some(Uint128::new(10)),
        },
    };

    let finish_execution = CallbackMsg::FinishExecution {
        destination: eris::ampz::DestinationRuntime::DepositAmplifier {
            receiver: None,
        },
        executor: Addr::unchecked("controller"),
    };

    execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: false,
            execution,
        },
    )
    .unwrap();

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("controller", &[]),
        ExecuteMsg::Execute {
            id: Uint128::new(1),
        },
    )
    .unwrap_err();

    assert_eq!(res, ContractError::BalanceLessThanThreshold {});

    deps.querier.bank_querier.update_balance("user", coins(105, CONTRACT_DENOM));

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("controller", &[]),
        ExecuteMsg::Execute {
            id: Uint128::new(1),
        },
    )
    .unwrap();

    // deposit + finish
    assert_eq!(
        res.messages[0].msg,
        CallbackMsg::AuthzDeposit {
            user_balance_start: vec![native_asset(CONTRACT_DENOM.to_string(), Uint128::new(100))],
            max_amount: Some(vec![native_asset(CONTRACT_DENOM.into(), Uint128::new(10))])
        }
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &user())
        .unwrap()
    );
    deps.querier.bank_querier.update_balance("user", coins(100, CONTRACT_DENOM));
    deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, coins(5, CONTRACT_DENOM));

    assert_eq!(
        res.messages[1].msg,
        finish_execution.into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &user()).unwrap()
    );

    finish_amplifier(&mut deps, "controller");
}

#[test]
fn check_execution_source_wallet_cw20_deposit_amplifier() {
    let mut deps = setup_test();
    deps.querier.set_cw20_balance("user", "astro", 50);

    let interval_s = 6 * HOUR;
    let execution = Execution {
        destination: eris::ampz::DestinationState::DepositAmplifier {
            receiver: None,
        },
        schedule: Schedule {
            interval_s,
            start: None,
        },
        user: "user".into(),
        source: eris::ampz::Source::Wallet {
            over: token_asset(astro(), Uint128::new(100)),
            max_amount: Some(Uint128::new(10)),
        },
    };

    execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: false,
            execution,
        },
    )
    .unwrap();

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("controller", &[]),
        ExecuteMsg::Execute {
            id: Uint128::new(1),
        },
    )
    .unwrap_err();

    assert_eq!(res, ContractError::BalanceLessThanThreshold {});

    deps.querier.set_cw20_balance("user", "astro", 105);

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("controller", &[]),
        ExecuteMsg::Execute {
            id: Uint128::new(1),
        },
    )
    .unwrap();

    // deposit + swap + finish
    assert_eq!(
        res.messages[0].msg,
        CallbackMsg::AuthzDeposit {
            user_balance_start: vec![token_asset(astro(), Uint128::new(100))],
            max_amount: Some(vec![token_asset(astro(), Uint128::new(10))])
        }
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &user())
        .unwrap()
    );

    deps.querier.set_cw20_balance("user", "astro", 100);
    deps.querier.set_cw20_balance(MOCK_CONTRACT_ADDR, "astro", 5);

    assert_eq!(
        res.messages[1].msg,
        CallbackMsg::Swap {
            asset_infos: vec![token_asset_info(astro())],
            into: native_asset_info(CONTRACT_DENOM.into())
        }
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &user())
        .unwrap()
    );

    deps.querier.set_cw20_balance(MOCK_CONTRACT_ADDR, "astro", 0);
    deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, coins(5, CONTRACT_DENOM));

    assert_eq!(
        res.messages[2].msg,
        CallbackMsg::FinishExecution {
            destination: eris::ampz::DestinationRuntime::DepositAmplifier {
                receiver: None,
            },
            executor: Addr::unchecked("controller"),
        }
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &user())
        .unwrap()
    );

    finish_amplifier(&mut deps, "controller");
}

#[test]
fn check_execution_source_astro_deposit_amplifier() {
    let mut deps = setup_test();
    deps.querier.bank_querier.update_balance("user", coins(100, CONTRACT_DENOM));
    deps.querier.set_cw20_balance("user", "astro", 1000);

    let interval_s = 6 * HOUR;
    let execution = Execution {
        destination: eris::ampz::DestinationState::DepositAmplifier {
            receiver: None,
        },
        schedule: Schedule {
            interval_s,
            start: None,
        },
        user: "user".into(),
        source: eris::ampz::Source::AstroRewards {
            lps: vec!["lp1".into(), "lp2".into()],
        },
    };

    execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: false,
            execution,
        },
    )
    .unwrap();

    let env = mock_env_at_timestamp(DAY);
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("controller", &[]),
        ExecuteMsg::Execute {
            id: Uint128::new(1),
        },
    )
    .unwrap();

    // claim_astro + deposit + swap + finish
    assert_eq!(
        res.messages[0].msg,
        Generator(Addr::unchecked("generator"))
            .claim_rewards_msg(vec!["lp1".into(), "lp2".into()])
            .unwrap()
            .to_authz_msg("user", &env)
            .unwrap()
    );
    // claim increases user balance
    deps.querier.bank_querier.update_balance("user", coins(120, CONTRACT_DENOM));
    deps.querier.set_cw20_balance("user", "astro", 1300);

    assert_eq!(
        res.messages[1].msg,
        CallbackMsg::AuthzDeposit {
            user_balance_start: vec![
                native_asset(CONTRACT_DENOM.into(), Uint128::new(100)),
                token_asset(astro(), Uint128::new(1000))
            ],
            max_amount: None
        }
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &user())
        .unwrap()
    );

    // deposit adds funds to the contract
    deps.querier.bank_querier.update_balance("user", coins(100, CONTRACT_DENOM));
    deps.querier.set_cw20_balance("user", "astro", 1000);

    deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, coins(20, CONTRACT_DENOM));
    deps.querier.set_cw20_balance(MOCK_CONTRACT_ADDR, "astro", 300);

    assert_eq!(
        res.messages[2].msg,
        CallbackMsg::Swap {
            asset_infos: vec![native_asset_info(CONTRACT_DENOM.into()), token_asset_info(astro())],
            into: native_asset_info(CONTRACT_DENOM.into())
        }
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &user())
        .unwrap()
    );

    deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, coins(50, CONTRACT_DENOM));
    deps.querier.set_cw20_balance(MOCK_CONTRACT_ADDR, "astro", 0);

    assert_eq!(
        res.messages[3].msg,
        CallbackMsg::FinishExecution {
            destination: eris::ampz::DestinationRuntime::DepositAmplifier {
                receiver: None,
            },
            executor: Addr::unchecked("controller"),
        }
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &user())
        .unwrap()
    );

    finish_amplifier(&mut deps, "controller");
}

#[test]
fn check_execution_source_wallet_cw20_repay() {
    let mut deps = setup_test();
    deps.querier.set_cw20_balance("user", "astro", 50);

    let interval_s = 6 * HOUR;
    let execution = Execution {
        destination: eris::ampz::DestinationState::Repay {
            market: eris::ampz::RepayMarket::Capapult,
        },
        schedule: Schedule {
            interval_s,
            start: None,
        },
        user: "user".into(),
        source: eris::ampz::Source::Wallet {
            over: token_asset(astro(), Uint128::new(5)),
            max_amount: Some(Uint128::new(50)),
        },
    };

    execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: false,
            execution,
        },
    )
    .unwrap();

    deps.querier.set_cw20_balance("user", "astro", 105);

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("controller", &[]),
        ExecuteMsg::Execute {
            id: Uint128::new(1),
        },
    )
    .unwrap();

    // deposit + swap + finish
    assert_eq!(
        res.messages[0].msg,
        CallbackMsg::AuthzDeposit {
            user_balance_start: vec![token_asset(astro(), Uint128::new(5))],
            max_amount: Some(vec![token_asset(astro(), Uint128::new(50))])
        }
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &user())
        .unwrap()
    );

    deps.querier.set_cw20_balance("user", "astro", 55);
    deps.querier.set_cw20_balance(MOCK_CONTRACT_ADDR, "astro", 50);

    assert_eq!(
        res.messages[1].msg,
        CallbackMsg::Swap {
            asset_infos: vec![token_asset_info(astro())],
            into: token_asset_info(solid())
        }
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &user())
        .unwrap()
    );

    deps.querier.set_cw20_balance(MOCK_CONTRACT_ADDR, "astro", 0);
    deps.querier.set_cw20_balance(MOCK_CONTRACT_ADDR, "solid", 500);

    let finish = CallbackMsg::FinishExecution {
        destination: eris::ampz::DestinationRuntime::Repay {
            market: eris::ampz::RepayMarket::Capapult,
        },
        executor: Addr::unchecked("controller"),
    };

    assert_eq!(
        res.messages[2].msg,
        finish.into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &user()).unwrap()
    );

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::Callback(finish.into_callback_wrapper(1, &user())),
    )
    .unwrap();

    assert_eq!(res.messages.len(), 3);

    assert_eq!(
        res.messages[0].msg,
        token_asset(solid(), Uint128::new(15)).transfer_msg(&fee_receiver()).unwrap()
    );

    assert_eq!(
        res.messages[1].msg,
        token_asset(solid(), Uint128::new(485)).transfer_msg(&user()).unwrap()
    );

    assert_eq!(
        res.messages[2].msg,
        CapapultMarket(Addr::unchecked("capapult_market"))
            .repay_loan(token_asset(solid(), Uint128::new(400)))
            .unwrap()
            .to_authz_msg(user(), &mock_env())
            .unwrap()
    );
}

#[test]
fn check_execution_source_wallet_cw20_deposit_collateral() {
    let mut deps = setup_test();

    let eriscw = Addr::unchecked("eriscw");
    let eriscw_assetinfo = token_asset_info(eriscw.clone());
    let interval_s = 6 * HOUR;
    let execution = Execution {
        destination: eris::ampz::DestinationState::DepositCollateral {
            market: eris::ampz::DepositMarket::Capapult {
                asset_info: eriscw_assetinfo.clone(),
            },
        },
        schedule: Schedule {
            interval_s,
            start: None,
        },
        user: "user".into(),
        source: eris::ampz::Source::Wallet {
            over: token_asset(astro(), Uint128::new(5)),
            max_amount: Some(Uint128::new(50)),
        },
    };

    execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: false,
            execution,
        },
    )
    .unwrap();

    deps.querier.set_cw20_balance("user", "astro", 105);

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("controller", &[]),
        ExecuteMsg::Execute {
            id: Uint128::new(1),
        },
    )
    .unwrap();

    // deposit + swap + finish
    assert_eq!(
        res.messages[0].msg,
        CallbackMsg::AuthzDeposit {
            user_balance_start: vec![token_asset(astro(), Uint128::new(5))],
            max_amount: Some(vec![token_asset(astro(), Uint128::new(50))])
        }
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &user())
        .unwrap()
    );

    deps.querier.set_cw20_balance("user", "astro", 55);
    deps.querier.set_cw20_balance(MOCK_CONTRACT_ADDR, "astro", 50);

    assert_eq!(
        res.messages[1].msg,
        CallbackMsg::Swap {
            asset_infos: vec![token_asset_info(astro())],
            into: eriscw_assetinfo.clone()
        }
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &user())
        .unwrap()
    );

    deps.querier.set_cw20_balance(MOCK_CONTRACT_ADDR, "astro", 0);
    deps.querier.set_cw20_balance(MOCK_CONTRACT_ADDR, "eriscw", 500);

    let finish = CallbackMsg::FinishExecution {
        destination: eris::ampz::DestinationRuntime::DepositCollateral {
            market: eris::ampz::DepositMarket::Capapult {
                asset_info: eriscw_assetinfo,
            },
        },
        executor: Addr::unchecked("controller"),
    };

    assert_eq!(
        res.messages[2].msg,
        finish.into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &user()).unwrap()
    );

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::Callback(finish.into_callback_wrapper(1, &user())),
    )
    .unwrap();

    assert_eq!(res.messages.len(), 3);

    assert_eq!(
        res.messages[0].msg,
        token_asset(eriscw.clone(), Uint128::new(15)).transfer_msg(&fee_receiver()).unwrap()
    );

    assert_eq!(
        res.messages[1].msg,
        token_asset(eriscw.clone(), Uint128::new(485)).transfer_msg(&user()).unwrap()
    );

    let msgs = CapapultLocker {
        overseer: Addr::unchecked("capapult_overseer"),
        custody: Addr::unchecked("capapult_custody"),
    }
    .deposit_and_lock_collateral(token_asset(eriscw, Uint128::new(485)))
    .unwrap();

    assert_eq!(res.messages[2].msg, msgs.to_authz_msg(user(), &mock_env()).unwrap());
}

#[test]
fn check_execution_source_wallet_deposit_collateral_same_asset() {
    let mut deps = setup_test();
    deps.querier.set_cw20_balance("user", "eriscw", 500);

    let eriscw = Addr::unchecked("eriscw");
    let eriscw_assetinfo = token_asset_info(eriscw.clone());
    let interval_s = 6 * HOUR;
    let execution = Execution {
        destination: eris::ampz::DestinationState::DepositCollateral {
            market: eris::ampz::DepositMarket::Capapult {
                asset_info: eriscw_assetinfo.clone(),
            },
        },
        schedule: Schedule {
            interval_s,
            start: None,
        },
        user: "user".into(),
        source: eris::ampz::Source::Wallet {
            over: token_asset(eriscw.clone(), Uint128::new(5)),
            max_amount: Some(Uint128::new(500)),
        },
    };

    execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: false,
            execution,
        },
    )
    .unwrap();

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("controller", &[]),
        ExecuteMsg::Execute {
            id: Uint128::new(1),
        },
    )
    .unwrap();

    // deposit + NO swap + finish
    assert_eq!(
        res.messages[0].msg,
        CallbackMsg::AuthzDeposit {
            user_balance_start: vec![token_asset(eriscw.clone(), Uint128::new(5))],
            max_amount: Some(vec![token_asset(eriscw.clone(), Uint128::new(500))])
        }
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &user())
        .unwrap()
    );

    deps.querier.set_cw20_balance("user", "eriscw", 5);
    deps.querier.set_cw20_balance(MOCK_CONTRACT_ADDR, "eriscw", 495);

    // NO SWAP HAPPENING

    let finish = CallbackMsg::FinishExecution {
        destination: eris::ampz::DestinationRuntime::DepositCollateral {
            market: eris::ampz::DepositMarket::Capapult {
                asset_info: eriscw_assetinfo,
            },
        },
        executor: Addr::unchecked("controller"),
    };

    assert_eq!(
        res.messages[1].msg,
        finish.into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &user()).unwrap()
    );

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::Callback(finish.into_callback_wrapper(1, &user())),
    )
    .unwrap();

    assert_eq!(res.messages.len(), 3);

    assert_eq!(
        res.messages[0].msg,
        token_asset(eriscw.clone(), Uint128::new(13)).transfer_msg(&fee_receiver()).unwrap()
    );

    assert_eq!(
        res.messages[1].msg,
        token_asset(eriscw.clone(), Uint128::new(482)).transfer_msg(&user()).unwrap()
    );

    let msgs = CapapultLocker {
        overseer: Addr::unchecked("capapult_overseer"),
        custody: Addr::unchecked("capapult_custody"),
    }
    .deposit_and_lock_collateral(token_asset(eriscw, Uint128::new(482)))
    .unwrap();

    assert_eq!(res.messages[2].msg, msgs.to_authz_msg(user(), &mock_env()).unwrap());
}

#[test]
fn check_execution_source_claim_deposit_arbvault() {
    let mut deps = setup_test();

    let interval_s = 6 * HOUR;
    let execution = Execution {
        destination: eris::ampz::DestinationState::DepositArbVault {
            receiver: None,
        },
        schedule: Schedule {
            interval_s,
            start: None,
        },
        user: "user".into(),
        source: eris::ampz::Source::Claim,
    };

    let finish_execution = CallbackMsg::FinishExecution {
        destination: eris::ampz::DestinationRuntime::DepositArbVault {
            receiver: None,
        },
        executor: Addr::unchecked("controller"),
    };

    execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: false,
            execution,
        },
    )
    .unwrap();

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("controller", &[]),
        ExecuteMsg::Execute {
            id: Uint128::new(1),
        },
    )
    .unwrap();

    // claim + deposit + finish
    assert_eq!(res.messages.len(), 3);
    assert_eq!(
        res.messages[0].msg,
        MsgExec {
            grantee: MOCK_CONTRACT_ADDR.to_string(),
            msgs: vec![MsgWithdrawDelegatorReward {
                delegator_address: "user".to_string(),
                validator_address: "val1".to_string(),
                special_fields: SpecialFields::default()
            }
            .to_any()
            .unwrap()],
            special_fields: SpecialFields::default()
        }
        .to_authz_cosmos_msg()
    );

    assert_eq!(
        res.messages[1].msg,
        CallbackMsg::AuthzDeposit {
            user_balance_start: vec![native_asset(CONTRACT_DENOM.to_string(), Uint128::new(0))],
            max_amount: None
        }
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &user())
        .unwrap()
    );

    assert_eq!(
        res.messages[2].msg,
        finish_execution.into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &user()).unwrap()
    );
}

#[test]
fn check_execution_source_wallet_cw20_deposit_arbvault() {
    let mut deps = setup_test();
    deps.querier.set_cw20_balance("user", "astro", 50);

    let interval_s = 6 * HOUR;
    let execution = Execution {
        destination: eris::ampz::DestinationState::DepositArbVault {
            receiver: None,
        },
        schedule: Schedule {
            interval_s,
            start: None,
        },
        user: "user".into(),
        source: eris::ampz::Source::Wallet {
            over: token_asset(astro(), Uint128::new(100)),
            max_amount: Some(Uint128::new(10)),
        },
    };

    execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: false,
            execution,
        },
    )
    .unwrap();

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("controller", &[]),
        ExecuteMsg::Execute {
            id: Uint128::new(1),
        },
    )
    .unwrap_err();

    assert_eq!(res, ContractError::BalanceLessThanThreshold {});

    deps.querier.set_cw20_balance("user", "astro", 105);

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("controller", &[]),
        ExecuteMsg::Execute {
            id: Uint128::new(1),
        },
    )
    .unwrap();

    // deposit + swap + finish
    assert_eq!(
        res.messages[0].msg,
        CallbackMsg::AuthzDeposit {
            user_balance_start: vec![token_asset(astro(), Uint128::new(100))],
            max_amount: Some(vec![token_asset(astro(), Uint128::new(10))])
        }
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &user())
        .unwrap()
    );

    deps.querier.set_cw20_balance("user", "astro", 100);
    deps.querier.set_cw20_balance(MOCK_CONTRACT_ADDR, "astro", 5);

    assert_eq!(
        res.messages[1].msg,
        CallbackMsg::Swap {
            asset_infos: vec![token_asset_info(astro())],
            into: native_asset_info(CONTRACT_DENOM.into())
        }
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &user())
        .unwrap()
    );

    deps.querier.set_cw20_balance(MOCK_CONTRACT_ADDR, "astro", 0);
    deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, coins(5, CONTRACT_DENOM));

    assert_eq!(
        res.messages[2].msg,
        CallbackMsg::FinishExecution {
            destination: eris::ampz::DestinationRuntime::DepositArbVault {
                receiver: None,
            },
            executor: Addr::unchecked("controller"),
        }
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &user())
        .unwrap()
    );

    finish_amplifier(&mut deps, "controller");
}
