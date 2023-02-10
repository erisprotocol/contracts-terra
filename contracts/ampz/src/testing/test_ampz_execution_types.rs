use cosmwasm_std::testing::mock_info;
use eris::adapters::generator::Generator;

use crate::protos::msgex::CosmosMsgEx;
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

#[test]
fn check_execution_source_claim_deposit_amplifier() {
    let mut deps = setup_test();

    let interval_s = 100;
    let execution = Execution {
        destination: eris::ampz::DestinationState::DepositAmplifier {},
        schedule: Schedule {
            interval_s,
            start: None,
        },
        user: "user".into(),
        source: eris::ampz::Source::Claim,
    };

    let finish_execution = CallbackMsg::FinishExecution {
        destination: eris::ampz::DestinationRuntime::DepositAmplifier {},
        executor: Addr::unchecked("controller"),
    };

    execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: false,
            execution,
        },
    )
    .unwrap();

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("controller", &[]),
        ExecuteMsg::Execute {
            id: 1,
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
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &Addr::unchecked("user"))
        .unwrap()
    );

    assert_eq!(
        res.messages[2].msg,
        finish_execution
            .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &Addr::unchecked("user"))
            .unwrap()
    );
}

#[test]
fn check_execution_source_wallet_native_deposit_amplifier() {
    let mut deps = setup_test();
    deps.querier.bank_querier.update_balance("user", coins(50, CONTRACT_DENOM));

    let interval_s = 100;
    let execution = Execution {
        destination: eris::ampz::DestinationState::DepositAmplifier {},
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
        destination: eris::ampz::DestinationRuntime::DepositAmplifier {},
        executor: Addr::unchecked("controller"),
    };

    execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: false,
            execution,
        },
    )
    .unwrap();

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("controller", &[]),
        ExecuteMsg::Execute {
            id: 1,
        },
    )
    .unwrap_err();

    assert_eq!(res, ContractError::BalanceLessThanThreshold {});

    deps.querier.bank_querier.update_balance("user", coins(105, CONTRACT_DENOM));

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("controller", &[]),
        ExecuteMsg::Execute {
            id: 1,
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
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &Addr::unchecked("user"))
        .unwrap()
    );
    deps.querier.bank_querier.update_balance("user", coins(100, CONTRACT_DENOM));
    deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, coins(5, CONTRACT_DENOM));

    assert_eq!(
        res.messages[1].msg,
        finish_execution
            .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &Addr::unchecked("user"))
            .unwrap()
    );

    finish_amplifier(&mut deps, "controller");
}

#[test]
fn check_execution_source_wallet_cw20_deposit_amplifier() {
    let mut deps = setup_test();
    deps.querier.set_cw20_balance("user", "astro", 50);

    let interval_s = 100;
    let execution = Execution {
        destination: eris::ampz::DestinationState::DepositAmplifier {},
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
        mock_env_at_timestamp(1000),
        mock_info("user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: false,
            execution,
        },
    )
    .unwrap();

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("controller", &[]),
        ExecuteMsg::Execute {
            id: 1,
        },
    )
    .unwrap_err();

    assert_eq!(res, ContractError::BalanceLessThanThreshold {});

    deps.querier.set_cw20_balance("user", "astro", 105);

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("controller", &[]),
        ExecuteMsg::Execute {
            id: 1,
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
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &Addr::unchecked("user"))
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
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &Addr::unchecked("user"))
        .unwrap()
    );

    deps.querier.set_cw20_balance(MOCK_CONTRACT_ADDR, "astro", 0);
    deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, coins(5, CONTRACT_DENOM));

    assert_eq!(
        res.messages[2].msg,
        CallbackMsg::FinishExecution {
            destination: eris::ampz::DestinationRuntime::DepositAmplifier {},
            executor: Addr::unchecked("controller"),
        }
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &Addr::unchecked("user"))
        .unwrap()
    );

    finish_amplifier(&mut deps, "controller");
}

#[test]
fn check_execution_source_astro_deposit_amplifier() {
    let mut deps = setup_test();
    deps.querier.bank_querier.update_balance("user", coins(100, CONTRACT_DENOM));
    deps.querier.set_cw20_balance("user", "astro", 1000);

    let interval_s = 100;
    let execution = Execution {
        destination: eris::ampz::DestinationState::DepositAmplifier {},
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
        mock_env_at_timestamp(1000),
        mock_info("user", &[]),
        ExecuteMsg::AddExecution {
            overwrite: false,
            execution,
        },
    )
    .unwrap();

    let env = mock_env_at_timestamp(1000);
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("controller", &[]),
        ExecuteMsg::Execute {
            id: 1,
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
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &Addr::unchecked("user"))
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
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &Addr::unchecked("user"))
        .unwrap()
    );

    deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, coins(50, CONTRACT_DENOM));
    deps.querier.set_cw20_balance(MOCK_CONTRACT_ADDR, "astro", 0);

    assert_eq!(
        res.messages[3].msg,
        CallbackMsg::FinishExecution {
            destination: eris::ampz::DestinationRuntime::DepositAmplifier {},
            executor: Addr::unchecked("controller"),
        }
        .into_cosmos_msg(&Addr::unchecked(MOCK_CONTRACT_ADDR), 1, &Addr::unchecked("user"))
        .unwrap()
    );

    finish_amplifier(&mut deps, "controller");
}
