use crate::astro_gov::{AstroGov, AstroGovUnchecked, Lock};
use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::mock_querier::{mock_dependencies, WasmMockQuerier};
use crate::model::{
    CallbackMsg, Config, Cw20HookMsg, ExecuteMsg, InstantiateMsg, PoolInfo, QueryMsg, RewardInfo,
    StakerInfoResponse, StakingState, UserInfoResponse,
};
use astroport::asset::token_asset;
use astroport::generator::{
    Cw20HookMsg as GeneratorCw20HookMsg, ExecuteMsg as GeneratorExecuteMsg, PendingTokenResponse,
    UserInfoV2,
};
use astroport::restricted_vector::RestrictedVector;
use astroport_governance::escrow_fee_distributor::ExecuteMsg as FeeExecuteMsg;
use astroport_governance::utils::{EPOCH_START, WEEK};
use astroport_governance::voting_escrow::{
    Cw20HookMsg as VotingCw20HookMsg, ExecuteMsg as VotingExecuteMsg,
};
use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, to_binary, Addr, CosmosMsg, Decimal, OwnedDeps, Response, StdError, Timestamp,
    Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use eris::adapters::generator::Generator;

const ASTRO_TOKEN: &str = "astro";
const REWARD_TOKEN: &str = "reward";
const REWARD_PROXY: &str = "reward_proxy";
const USER1: &str = "user1";
const USER2: &str = "user2";
const USER3: &str = "user3";
const GENERATOR: &str = "generator";
const CONTROLLER: &str = "controller";
const FEE_COLLECTOR: &str = "fee_collector";
const LP_TOKEN: &str = "lp_token";

const XASTRO_TOKEN: &str = "xastro";
const VOTING_ESCROW: &str = "voting_escrow";
const FEE_DISTRIBUTOR: &str = "fee_distributor";
const GENERATOR_CONTROLLER: &str = "generator_controller";

#[test]
fn test() -> Result<(), ContractError> {
    let mut deps = mock_dependencies();
    create(&mut deps)?;
    config(&mut deps)?;

    deposit(&mut deps)?;
    claim_rewards(&mut deps)?;
    withdraw(&mut deps)?;

    stake(&mut deps)?;
    unstake(&mut deps)?;
    claim_income(&mut deps)?;

    send_income(&mut deps)?;

    Ok(())
}

fn assert_error(res: Result<Response, ContractError>, expected: &str) {
    match res {
        Err(ContractError::Std(StdError::GenericErr {
            msg,
            ..
        })) => assert_eq!(expected, msg),
        Err(err) => assert_eq!(expected, format!("{}", err)),
        _ => panic!("Expected exception"),
    }
}

#[allow(clippy::redundant_clone)]
fn create(
    deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
) -> Result<(), ContractError> {
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(EPOCH_START);

    let info = mock_info(USER1, &[]);
    let msg = InstantiateMsg {
        astro_token: ASTRO_TOKEN.to_string(),
        astro_gov: AstroGovUnchecked {
            xastro_token: XASTRO_TOKEN.to_string(),
            voting_escrow: VOTING_ESCROW.to_string(),
            fee_distributor: FEE_DISTRIBUTOR.to_string(),
            generator_controller: GENERATOR_CONTROLLER.to_string(),
        },
        owner: USER1.to_string(),
        generator: GENERATOR.to_string(),
        controller: CONTROLLER.to_string(),
        fee_collector: FEE_COLLECTOR.to_string(),
        staker_rate: Decimal::percent(160),
        max_quota: Uint128::from(1000u128),
        boost_fee: Decimal::percent(20),
    };
    let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg);
    assert_error(res, "staker_rate cannot greater than 1");

    let msg = InstantiateMsg {
        astro_token: ASTRO_TOKEN.to_string(),
        astro_gov: AstroGovUnchecked {
            xastro_token: XASTRO_TOKEN.to_string(),
            voting_escrow: VOTING_ESCROW.to_string(),
            fee_distributor: FEE_DISTRIBUTOR.to_string(),
            generator_controller: GENERATOR_CONTROLLER.to_string(),
        },
        owner: USER1.to_string(),
        generator: GENERATOR.to_string(),
        controller: CONTROLLER.to_string(),
        fee_collector: FEE_COLLECTOR.to_string(),
        staker_rate: Decimal::percent(50),
        max_quota: Uint128::from(1000u128),
        boost_fee: Decimal::percent(10),
    };
    let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg);
    assert!(res.is_ok());

    Ok(())
}

#[allow(clippy::redundant_clone)]
fn config(
    deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
) -> Result<(), ContractError> {
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(EPOCH_START);

    let info = mock_info(USER2, &[]);
    let msg = ExecuteMsg::UpdateConfig {
        controller: None,
        boost_fee: Some(Decimal::percent(120)),
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert_error(res, "Unauthorized");

    let info = mock_info(USER1, &[]);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    assert_error(res, "boost_fee cannot greater than 1");

    let msg = ExecuteMsg::UpdateConfig {
        controller: None,
        boost_fee: Some(Decimal::percent(20)),
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert!(res.is_ok());

    let msg = ExecuteMsg::UpdateParameters {
        max_quota: None,
        staker_rate: Some(Decimal::percent(160)),
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert_error(res, "Unauthorized");

    let info = mock_info(CONTROLLER, &[]);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    assert_error(res, "staker_rate cannot greater than 1");

    let msg = ExecuteMsg::UpdateParameters {
        max_quota: None,
        staker_rate: Some(Decimal::percent(60)),
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert!(res.is_ok());

    let msg = QueryMsg::Config {};
    let res: Config = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        Config {
            astro_token: Addr::unchecked(ASTRO_TOKEN),
            astro_gov: AstroGov {
                xastro_token: Addr::unchecked(XASTRO_TOKEN),
                voting_escrow: Addr::unchecked(VOTING_ESCROW),
                fee_distributor: Addr::unchecked(FEE_DISTRIBUTOR),
                generator_controller: Addr::unchecked(GENERATOR_CONTROLLER),
            },
            owner: Addr::unchecked(USER1),
            generator: Generator(Addr::unchecked(GENERATOR)),
            controller: Addr::unchecked(CONTROLLER),
            fee_collector: Addr::unchecked(FEE_COLLECTOR),
            staker_rate: Decimal::percent(60),
            max_quota: Uint128::from(1000u128),
            boost_fee: Decimal::percent(20),
        }
    );

    Ok(())
}

#[allow(clippy::redundant_clone)]
fn deposit(
    deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
) -> Result<(), ContractError> {
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(EPOCH_START);

    let msg = QueryMsg::Deposit {
        lp_token: LP_TOKEN.to_string(),
        user: USER1.to_string(),
    };
    let res: Uint128 = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(res, Uint128::zero());

    let msg = QueryMsg::PendingToken {
        lp_token: LP_TOKEN.to_string(),
        user: USER1.to_string(),
    };
    let res: PendingTokenResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(res.pending, Uint128::zero());
    assert_eq!(res.pending_on_proxy, None);

    let info = mock_info(LP_TOKEN, &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: USER1.to_string(),
        amount: Uint128::from(100u128),
        msg: to_binary(&Cw20HookMsg::Deposit {})?,
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: MOCK_CONTRACT_ADDR.to_string(),
            msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::Deposit {
                amount: Uint128::from(100u128),
                staker_addr: Addr::unchecked(USER1),
                lp_token: Addr::unchecked(LP_TOKEN),
            }))?,
            funds: vec![],
        }),]
    );

    let msg = ExecuteMsg::Callback(CallbackMsg::Deposit {
        amount: Uint128::from(100u128),
        staker_addr: Addr::unchecked(USER1),
        lp_token: Addr::unchecked(LP_TOKEN),
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert_error(res, "Callbacks cannot be invoked externally");

    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: LP_TOKEN.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: GENERATOR.to_string(),
                    amount: Uint128::from(100u128),
                    msg: to_binary(&GeneratorCw20HookMsg::Deposit {})?,
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::AfterBondChanged {
                    lp_token: Addr::unchecked(LP_TOKEN),
                }))?,
                funds: vec![],
            }),
        ]
    );
    deps.querier.set_balance(GENERATOR.to_string(), LP_TOKEN.to_string(), Uint128::from(100u128));
    deps.querier
        .set_reward_proxy(&Addr::unchecked(REWARD_PROXY), &Addr::unchecked(REWARD_TOKEN))?;
    deps.querier.set_user_info(
        &Addr::unchecked(LP_TOKEN),
        &Addr::unchecked(MOCK_CONTRACT_ADDR),
        &UserInfoV2 {
            amount: Uint128::from(100u128),
            reward_user_index: Decimal::zero(),
            reward_debt_proxy: RestrictedVector::from(vec![(
                Addr::unchecked(REWARD_PROXY),
                Uint128::zero(),
            )]),
            virtual_amount: Uint128::from(80u128),
        },
    )?;

    let msg = ExecuteMsg::Callback(CallbackMsg::AfterBondChanged {
        lp_token: Addr::unchecked(LP_TOKEN),
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert!(res.is_ok());

    let msg = QueryMsg::UserInfo {
        lp_token: LP_TOKEN.to_string(),
        user: USER1.to_string(),
    };
    let res: UserInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        UserInfoResponse {
            bond_share: Uint128::from(100u128),
            bond_amount: Uint128::from(100u128),
            reward_indexes: RestrictedVector::default(),
            pending_rewards: RestrictedVector::default(),
        }
    );

    let info = mock_info(LP_TOKEN, &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: USER2.to_string(),
        amount: Uint128::from(60u128),
        msg: to_binary(&Cw20HookMsg::Deposit {})?,
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: GENERATOR.to_string(),
                msg: to_binary(&GeneratorExecuteMsg::Withdraw {
                    lp_token: LP_TOKEN.to_string(),
                    amount: Uint128::zero()
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::AfterBondClaimed {
                    lp_token: Addr::unchecked(LP_TOKEN),
                    prev_balances: vec![
                        (Addr::unchecked(ASTRO_TOKEN), Uint128::zero()),
                        (Addr::unchecked(REWARD_TOKEN), Uint128::zero()),
                    ]
                }))?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::Deposit {
                    amount: Uint128::from(60u128),
                    staker_addr: Addr::unchecked(USER2),
                    lp_token: Addr::unchecked(LP_TOKEN),
                }))?,
                funds: vec![],
            }),
        ]
    );
    deps.querier.set_balance(
        ASTRO_TOKEN.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        Uint128::from(10u128),
    );
    deps.querier.set_balance(
        REWARD_TOKEN.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        Uint128::from(20u128),
    );
    deps.querier.set_user_info(
        &Addr::unchecked(LP_TOKEN),
        &Addr::unchecked(MOCK_CONTRACT_ADDR),
        &UserInfoV2 {
            amount: Uint128::from(100u128),
            reward_user_index: Decimal::permille(125),
            reward_debt_proxy: RestrictedVector::from(vec![(
                Addr::unchecked(REWARD_PROXY),
                Uint128::from(20u128),
            )]),
            virtual_amount: Uint128::from(80u128),
        },
    )?;

    let msg = ExecuteMsg::Callback(CallbackMsg::AfterBondClaimed {
        lp_token: Addr::unchecked(LP_TOKEN),
        prev_balances: vec![
            (Addr::unchecked(ASTRO_TOKEN), Uint128::zero()),
            (Addr::unchecked(REWARD_TOKEN), Uint128::zero()),
        ],
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert_error(res, "Callbacks cannot be invoked externally");

    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    assert!(res.is_ok());

    let msg = QueryMsg::PoolInfo {
        lp_token: LP_TOKEN.to_string(),
    };
    let res: PoolInfo = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        PoolInfo {
            total_bond_share: Uint128::from(100u128),
            reward_indexes: RestrictedVector::from(vec![
                (Addr::unchecked(ASTRO_TOKEN), Decimal::percent(7)),
                (Addr::unchecked(REWARD_TOKEN), Decimal::percent(20)),
            ]),
            prev_reward_user_index: Decimal::permille(125),
            prev_reward_debt_proxy: RestrictedVector::from(vec![(
                Addr::unchecked(REWARD_TOKEN),
                Uint128::from(20u128)
            ),]),
        }
    );

    let msg = QueryMsg::RewardInfo {
        token: ASTRO_TOKEN.to_string(),
    };
    let res: RewardInfo = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        RewardInfo {
            staker_income: Uint128::from(2u128),
            fee: Uint128::from(1u128),
            reconciled_amount: Uint128::from(10u128),
        }
    );

    let msg = QueryMsg::RewardInfo {
        token: REWARD_TOKEN.to_string(),
    };
    let res: RewardInfo = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        RewardInfo {
            staker_income: Uint128::zero(),
            fee: Uint128::zero(),
            reconciled_amount: Uint128::from(20u128),
        }
    );

    let msg = ExecuteMsg::Callback(CallbackMsg::Deposit {
        amount: Uint128::from(60u128),
        staker_addr: Addr::unchecked(USER2),
        lp_token: Addr::unchecked(LP_TOKEN),
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: LP_TOKEN.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: GENERATOR.to_string(),
                    amount: Uint128::from(60u128),
                    msg: to_binary(&GeneratorCw20HookMsg::Deposit {})?,
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::AfterBondChanged {
                    lp_token: Addr::unchecked(LP_TOKEN),
                }))?,
                funds: vec![],
            }),
        ]
    );

    deps.querier.set_balance(GENERATOR.to_string(), LP_TOKEN.to_string(), Uint128::from(160u128));
    deps.querier.set_user_info(
        &Addr::unchecked(LP_TOKEN),
        &Addr::unchecked(MOCK_CONTRACT_ADDR),
        &UserInfoV2 {
            amount: Uint128::from(160u128),
            reward_user_index: Decimal::permille(125),
            reward_debt_proxy: RestrictedVector::default(),
            virtual_amount: Uint128::from(128u128),
        },
    )?;

    let msg = QueryMsg::UserInfo {
        lp_token: LP_TOKEN.to_string(),
        user: USER2.to_string(),
    };
    let res: UserInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        UserInfoResponse {
            bond_share: Uint128::from(60u128),
            bond_amount: Uint128::from(60u128),
            reward_indexes: RestrictedVector::from(vec![
                (Addr::unchecked(ASTRO_TOKEN), Decimal::percent(7)),
                (Addr::unchecked(REWARD_TOKEN), Decimal::percent(20)),
            ]),
            pending_rewards: RestrictedVector::from(vec![
                (Addr::unchecked(ASTRO_TOKEN), Uint128::zero()),
                (Addr::unchecked(REWARD_TOKEN), Uint128::zero()),
            ]),
        }
    );

    let msg = ExecuteMsg::Callback(CallbackMsg::AfterBondChanged {
        lp_token: Addr::unchecked(LP_TOKEN),
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert!(res.is_ok());

    let msg = QueryMsg::PoolInfo {
        lp_token: LP_TOKEN.to_string(),
    };
    let res: PoolInfo = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        PoolInfo {
            total_bond_share: Uint128::from(160u128),
            reward_indexes: RestrictedVector::from(vec![
                (Addr::unchecked(ASTRO_TOKEN), Decimal::percent(7)),
                (Addr::unchecked(REWARD_TOKEN), Decimal::percent(20)),
            ]),
            prev_reward_user_index: Decimal::permille(125),
            prev_reward_debt_proxy: RestrictedVector::default(),
        }
    );

    Ok(())
}

#[allow(clippy::redundant_clone)]
fn claim_rewards(
    deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
) -> Result<(), ContractError> {
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(EPOCH_START);

    deps.querier.set_balance(GENERATOR.to_string(), ASTRO_TOKEN.to_string(), Uint128::from(32u128));
    deps.querier.set_balance(
        GENERATOR.to_string(),
        REWARD_TOKEN.to_string(),
        Uint128::from(16u128),
    );
    deps.querier.set_user_info(
        &Addr::unchecked(LP_TOKEN),
        &Addr::unchecked(MOCK_CONTRACT_ADDR),
        &UserInfoV2 {
            amount: Uint128::from(160u128),
            reward_user_index: Decimal::permille(125),
            reward_debt_proxy: RestrictedVector::from(vec![(
                Addr::unchecked(REWARD_PROXY),
                Uint128::zero(),
            )]),
            virtual_amount: Uint128::from(160u128),
        },
    )?;

    let msg = QueryMsg::Deposit {
        lp_token: LP_TOKEN.to_string(),
        user: USER1.to_string(),
    };
    let res: Uint128 = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(res, Uint128::from(100u128));

    let msg = QueryMsg::PendingToken {
        lp_token: LP_TOKEN.to_string(),
        user: USER1.to_string(),
    };
    let res: PendingTokenResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(res.pending, Uint128::from(18u128));
    assert_eq!(
        res.pending_on_proxy,
        Some(vec![token_asset(Addr::unchecked(REWARD_TOKEN), Uint128::from(30u128)),])
    );

    let info = mock_info(USER1, &[]);
    let msg = ExecuteMsg::ClaimRewards {
        lp_tokens: vec![LP_TOKEN.to_string()],
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: GENERATOR.to_string(),
                msg: to_binary(&GeneratorExecuteMsg::Withdraw {
                    lp_token: LP_TOKEN.to_string(),
                    amount: Uint128::zero()
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::AfterBondClaimed {
                    lp_token: Addr::unchecked(LP_TOKEN),
                    prev_balances: vec![
                        (Addr::unchecked(ASTRO_TOKEN), Uint128::from(10u128)),
                        (Addr::unchecked(REWARD_TOKEN), Uint128::from(20u128)),
                    ]
                }))?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::ClaimRewards {
                    lp_token: Addr::unchecked(LP_TOKEN),
                    staker_addr: Addr::unchecked(USER1),
                }))?,
                funds: vec![],
            }),
        ]
    );
    deps.querier.set_balance(
        ASTRO_TOKEN.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        Uint128::from(42u128),
    );
    deps.querier.set_balance(
        REWARD_TOKEN.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        Uint128::from(36u128),
    );
    deps.querier.set_user_info(
        &Addr::unchecked(LP_TOKEN),
        &Addr::unchecked(MOCK_CONTRACT_ADDR),
        &UserInfoV2 {
            amount: Uint128::from(160u128),
            reward_user_index: Decimal::permille(325),
            reward_debt_proxy: RestrictedVector::from(vec![(
                Addr::unchecked(REWARD_PROXY),
                Uint128::from(16u128),
            )]),
            virtual_amount: Uint128::from(160u128),
        },
    )?;

    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let msg = ExecuteMsg::Callback(CallbackMsg::AfterBondClaimed {
        lp_token: Addr::unchecked(LP_TOKEN),
        prev_balances: vec![
            (Addr::unchecked(ASTRO_TOKEN), Uint128::from(10u128)),
            (Addr::unchecked(REWARD_TOKEN), Uint128::from(20u128)),
        ],
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    assert!(res.is_ok());

    let msg = QueryMsg::PoolInfo {
        lp_token: LP_TOKEN.to_string(),
    };
    let res: PoolInfo = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        PoolInfo {
            total_bond_share: Uint128::from(160u128),
            reward_indexes: RestrictedVector::from(vec![
                (Addr::unchecked(ASTRO_TOKEN), Decimal::from_ratio(18875u128, 100000u128)),
                (Addr::unchecked(REWARD_TOKEN), Decimal::percent(30)),
            ]),
            prev_reward_user_index: Decimal::permille(325),
            prev_reward_debt_proxy: RestrictedVector::from(vec![(
                Addr::unchecked(REWARD_TOKEN),
                Uint128::from(16u128)
            ),]),
        }
    );

    let msg = QueryMsg::RewardInfo {
        token: ASTRO_TOKEN.to_string(),
    };
    let res: RewardInfo = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        RewardInfo {
            staker_income: Uint128::from(11u128),
            fee: Uint128::from(5u128),
            reconciled_amount: Uint128::from(42u128),
        }
    );

    let msg = QueryMsg::RewardInfo {
        token: REWARD_TOKEN.to_string(),
    };
    let res: RewardInfo = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        RewardInfo {
            staker_income: Uint128::zero(),
            fee: Uint128::zero(),
            reconciled_amount: Uint128::from(36u128),
        }
    );

    let msg = ExecuteMsg::Callback(CallbackMsg::ClaimRewards {
        lp_token: Addr::unchecked(LP_TOKEN),
        staker_addr: Addr::unchecked(USER1),
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: ASTRO_TOKEN.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: USER1.to_string(),
                    amount: Uint128::from(18u128),
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: REWARD_TOKEN.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: USER1.to_string(),
                    amount: Uint128::from(30u128),
                })?,
                funds: vec![],
            }),
        ]
    );

    let msg = QueryMsg::UserInfo {
        lp_token: LP_TOKEN.to_string(),
        user: USER1.to_string(),
    };
    let res: UserInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        UserInfoResponse {
            bond_share: Uint128::from(100u128),
            bond_amount: Uint128::from(100u128),
            reward_indexes: RestrictedVector::from(vec![
                (Addr::unchecked(ASTRO_TOKEN), Decimal::from_ratio(18875u128, 100000u128)),
                (Addr::unchecked(REWARD_TOKEN), Decimal::percent(30)),
            ]),
            pending_rewards: RestrictedVector::from(vec![
                (Addr::unchecked(ASTRO_TOKEN), Uint128::zero()),
                (Addr::unchecked(REWARD_TOKEN), Uint128::zero()),
            ]),
        }
    );

    let msg = QueryMsg::RewardInfo {
        token: ASTRO_TOKEN.to_string(),
    };
    let res: RewardInfo = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        RewardInfo {
            staker_income: Uint128::from(11u128),
            fee: Uint128::from(5u128),
            reconciled_amount: Uint128::from(24u128),
        }
    );

    let msg = QueryMsg::RewardInfo {
        token: REWARD_TOKEN.to_string(),
    };
    let res: RewardInfo = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        RewardInfo {
            staker_income: Uint128::zero(),
            fee: Uint128::zero(),
            reconciled_amount: Uint128::from(6u128),
        }
    );

    Ok(())
}

#[allow(clippy::redundant_clone)]
fn withdraw(
    deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
) -> Result<(), ContractError> {
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(EPOCH_START);
    let info = mock_info(USER1, &[]);

    let msg = ExecuteMsg::Withdraw {
        lp_token: LP_TOKEN.to_string(),
        amount: Uint128::from(100u128),
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: GENERATOR.to_string(),
                msg: to_binary(&GeneratorExecuteMsg::Withdraw {
                    lp_token: LP_TOKEN.to_string(),
                    amount: Uint128::zero()
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::AfterBondClaimed {
                    lp_token: Addr::unchecked(LP_TOKEN),
                    prev_balances: vec![
                        (Addr::unchecked(ASTRO_TOKEN), Uint128::from(42u128)),
                        (Addr::unchecked(REWARD_TOKEN), Uint128::from(36u128)),
                    ]
                }))?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::Withdraw {
                    amount: Uint128::from(100u128),
                    staker_addr: Addr::unchecked(USER1),
                    lp_token: Addr::unchecked(LP_TOKEN),
                }))?,
                funds: vec![],
            }),
        ]
    );

    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let msg = ExecuteMsg::Callback(CallbackMsg::AfterBondClaimed {
        lp_token: Addr::unchecked(LP_TOKEN),
        prev_balances: vec![
            (Addr::unchecked(ASTRO_TOKEN), Uint128::from(42u128)),
            (Addr::unchecked(REWARD_TOKEN), Uint128::from(36u128)),
        ],
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    assert!(res.is_ok());

    let msg = ExecuteMsg::Callback(CallbackMsg::Withdraw {
        amount: Uint128::from(100u128),
        staker_addr: Addr::unchecked(USER3),
        lp_token: Addr::unchecked(LP_TOKEN),
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    assert_error(res, "eris_generator_proxy::model::UserInfo not found");

    let msg = ExecuteMsg::Callback(CallbackMsg::Withdraw {
        amount: Uint128::from(101u128),
        staker_addr: Addr::unchecked(USER1),
        lp_token: Addr::unchecked(LP_TOKEN),
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    assert_error(res, "Cannot Sub with 100 and 101");

    let msg = ExecuteMsg::Callback(CallbackMsg::Withdraw {
        amount: Uint128::from(100u128),
        staker_addr: Addr::unchecked(USER1),
        lp_token: Addr::unchecked(LP_TOKEN),
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: GENERATOR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::Withdraw {
                    lp_token: LP_TOKEN.to_string(),
                    amount: Uint128::from(100u128),
                })?,
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: LP_TOKEN.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: USER1.to_string(),
                    amount: Uint128::from(100u128),
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::AfterBondChanged {
                    lp_token: Addr::unchecked(LP_TOKEN),
                }))?,
                funds: vec![],
            }),
        ]
    );
    deps.querier.set_balance(GENERATOR.to_string(), LP_TOKEN.to_string(), Uint128::from(60u128));
    deps.querier.set_user_info(
        &Addr::unchecked(LP_TOKEN),
        &Addr::unchecked(MOCK_CONTRACT_ADDR),
        &UserInfoV2 {
            amount: Uint128::from(60u128),
            reward_user_index: Decimal::permille(325),
            reward_debt_proxy: RestrictedVector::default(),
            virtual_amount: Uint128::from(60u128),
        },
    )?;

    let msg = ExecuteMsg::Callback(CallbackMsg::AfterBondChanged {
        lp_token: Addr::unchecked(LP_TOKEN),
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert!(res.is_ok());

    let msg = QueryMsg::PoolInfo {
        lp_token: LP_TOKEN.to_string(),
    };
    let res: PoolInfo = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        PoolInfo {
            total_bond_share: Uint128::from(60u128),
            reward_indexes: RestrictedVector::from(vec![
                (Addr::unchecked(ASTRO_TOKEN), Decimal::from_ratio(18875u128, 100000u128)),
                (Addr::unchecked(REWARD_TOKEN), Decimal::percent(30)),
            ]),
            prev_reward_user_index: Decimal::permille(325),
            prev_reward_debt_proxy: RestrictedVector::default(),
        }
    );

    let msg = QueryMsg::UserInfo {
        lp_token: LP_TOKEN.to_string(),
        user: USER1.to_string(),
    };
    let res: UserInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        UserInfoResponse {
            bond_share: Uint128::zero(),
            bond_amount: Uint128::zero(),
            reward_indexes: RestrictedVector::from(vec![
                (Addr::unchecked(ASTRO_TOKEN), Decimal::from_ratio(18875u128, 100000u128)),
                (Addr::unchecked(REWARD_TOKEN), Decimal::percent(30)),
            ]),
            pending_rewards: RestrictedVector::from(vec![
                (Addr::unchecked(ASTRO_TOKEN), Uint128::zero()),
                (Addr::unchecked(REWARD_TOKEN), Uint128::zero()),
            ]),
        }
    );

    Ok(())
}

#[allow(clippy::redundant_clone)]
fn stake(deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>) -> Result<(), ContractError> {
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(EPOCH_START);
    let info = mock_info(USER1, &[]);

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: USER1.to_string(),
        amount: Uint128::from(1001u128),
        msg: to_binary(&Cw20HookMsg::Stake {})?,
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    assert_error(res, "Unauthorized");

    let info = mock_info(XASTRO_TOKEN, &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: USER1.to_string(),
        amount: Uint128::from(800u128),
        msg: to_binary(&Cw20HookMsg::Stake {})?,
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: XASTRO_TOKEN.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: VOTING_ESCROW.to_string(),
                amount: Uint128::from(800u128),
                msg: to_binary(&VotingCw20HookMsg::CreateLock {
                    time: WEEK,
                })?,
            })?,
            funds: vec![],
        }),]
    );
    deps.querier.set_lock(
        Addr::unchecked(MOCK_CONTRACT_ADDR),
        &Lock {
            amount: Uint128::from(800u128),
            start: 0,
            end: 1,
            last_extend_lock_period: 0,
        },
    )?;

    let msg = QueryMsg::RewardInfo {
        token: ASTRO_TOKEN.to_string(),
    };
    let res: RewardInfo = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        RewardInfo {
            staker_income: Uint128::from(11u128),
            fee: Uint128::from(5u128),
            reconciled_amount: Uint128::from(24u128),
        }
    );

    let msg = QueryMsg::StakingState {};
    let res: StakingState = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        StakingState {
            total_bond_share: Uint128::from(800u128),
            reward_index: Decimal::zero(),
            next_claim_period: 0,
            total_unstaking_amount: Uint128::zero(),
            total_unstaked_amount: Uint128::zero(),
            unstaking_period: 0
        }
    );

    let msg = QueryMsg::StakerInfo {
        user: USER1.to_string(),
    };
    let res: StakerInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        StakerInfoResponse {
            bond_share: Uint128::from(800u128),
            bond_amount: Uint128::from(800u128),
            reward_index: Decimal::from_ratio(1375u128, 100000u128),
            pending_reward: Uint128::from(11u128),
            unstaking_amount: Uint128::zero(),
            unstaked_amount: Uint128::zero(),
            unstaking_period: 0
        }
    );

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: USER2.to_string(),
        amount: Uint128::from(400u128),
        msg: to_binary(&Cw20HookMsg::Stake {})?,
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    assert_error(res, "Exceed quota, remaining quota is 200");

    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: USER2.to_string(),
        amount: Uint128::from(100u128),
        msg: to_binary(&Cw20HookMsg::Stake {})?,
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: XASTRO_TOKEN.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: VOTING_ESCROW.to_string(),
                amount: Uint128::from(100u128),
                msg: to_binary(&VotingCw20HookMsg::ExtendLockAmount {})?,
            })?,
            funds: vec![],
        }),]
    );
    deps.querier.set_lock(
        Addr::unchecked(MOCK_CONTRACT_ADDR),
        &Lock {
            amount: Uint128::from(900u128),
            start: 0,
            end: 1,
            last_extend_lock_period: 0,
        },
    )?;

    Ok(())
}

#[allow(clippy::redundant_clone)]
fn unstake(
    deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
) -> Result<(), ContractError> {
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(EPOCH_START);
    let info = mock_info(USER1, &[]);

    let msg = ExecuteMsg::RequestUnstake {
        amount: Uint128::from(500u128),
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    assert!(res.is_ok());

    let msg = QueryMsg::StakingState {};
    let res: StakingState = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        StakingState {
            total_bond_share: Uint128::from(400u128),
            reward_index: Decimal::from_ratio(1375u128, 100000u128),
            next_claim_period: 0,
            total_unstaking_amount: Uint128::from(500u128),
            total_unstaked_amount: Uint128::zero(),
            unstaking_period: 0
        }
    );

    let msg = QueryMsg::StakerInfo {
        user: USER1.to_string(),
    };
    let res: StakerInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        StakerInfoResponse {
            bond_share: Uint128::from(300u128),
            bond_amount: Uint128::from(300u128),
            reward_index: Decimal::from_ratio(1375u128, 100000u128),
            pending_reward: Uint128::from(11u128),
            unstaking_amount: Uint128::from(500u128),
            unstaked_amount: Uint128::zero(),
            unstaking_period: 0
        }
    );

    let msg = ExecuteMsg::RequestUnstake {
        amount: Uint128::from(500u128),
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    assert_error(res, "Cannot Sub with 300 and 500");

    let info = mock_info(XASTRO_TOKEN, &[]);
    env.block.time = Timestamp::from_seconds(EPOCH_START + WEEK);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: USER2.to_string(),
        amount: Uint128::from(100u128),
        msg: to_binary(&Cw20HookMsg::Stake {})?,
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                msg: to_binary(&ExecuteMsg::Relock {})?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: XASTRO_TOKEN.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: VOTING_ESCROW.to_string(),
                    amount: Uint128::from(100u128),
                    msg: to_binary(&VotingCw20HookMsg::ExtendLockAmount {})?,
                })?,
                funds: vec![],
            }),
        ]
    );

    let msg = ExecuteMsg::Relock {};
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert_error(res, "Unauthorized");

    deps.querier.set_rewards_per_week(0u64, Uint128::from(500u128))?;
    deps.querier.set_last_claim_period(Addr::unchecked(MOCK_CONTRACT_ADDR), 1u64)?;
    deps.querier.set_balance(
        VOTING_ESCROW.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        Uint128::from(1000u128),
    );
    deps.querier.set_balance(
        VOTING_ESCROW.to_string(),
        VOTING_ESCROW.to_string(),
        Uint128::from(10000u128),
    );
    deps.querier.set_balance(
        ASTRO_TOKEN.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        Uint128::from(92u128),
    );
    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: FEE_DISTRIBUTOR.to_string(),
                msg: to_binary(&FeeExecuteMsg::Claim {
                    recipient: None,
                    max_periods: None,
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::AfterStakingClaimed {
                    prev_balance: Uint128::from(92u128),
                }))?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: VOTING_ESCROW.to_string(),
                msg: to_binary(&VotingExecuteMsg::Withdraw {})?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: XASTRO_TOKEN.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: VOTING_ESCROW.to_string(),
                    amount: Uint128::from(400u128),
                    msg: to_binary(&VotingCw20HookMsg::CreateLock {
                        time: WEEK,
                    })?,
                })?,
                funds: vec![],
            })
        ]
    );

    let msg = ExecuteMsg::Callback(CallbackMsg::AfterStakingClaimed {
        prev_balance: Uint128::from(92u128),
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    assert!(res.is_ok());
    deps.querier.set_lock(
        Addr::unchecked(MOCK_CONTRACT_ADDR),
        &Lock {
            amount: Uint128::from(500u128),
            start: 1,
            end: 2,
            last_extend_lock_period: 1,
        },
    )?;

    let msg = QueryMsg::RewardInfo {
        token: ASTRO_TOKEN.to_string(),
    };
    let res: RewardInfo = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        RewardInfo {
            staker_income: Uint128::zero(),
            fee: Uint128::from(5u128),
            reconciled_amount: Uint128::from(74u128),
        }
    );

    let msg = QueryMsg::StakingState {};
    let res: StakingState = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        StakingState {
            total_bond_share: Uint128::from(500u128),
            reward_index: Decimal::from_ratio(11375u128, 100000u128),
            next_claim_period: 1,
            total_unstaking_amount: Uint128::zero(),
            total_unstaked_amount: Uint128::from(500u128),
            unstaking_period: 1,
        }
    );

    let info = mock_info(USER1, &[]);
    let msg = ExecuteMsg::RequestUnstake {
        amount: Uint128::from(300u128),
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    assert!(res.is_ok());

    let msg = QueryMsg::StakerInfo {
        user: USER1.to_string(),
    };
    let res: StakerInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        StakerInfoResponse {
            bond_share: Uint128::zero(),
            bond_amount: Uint128::zero(),
            reward_index: Decimal::from_ratio(11375u128, 100000u128),
            pending_reward: Uint128::from(41u128),
            unstaking_amount: Uint128::from(300u128),
            unstaked_amount: Uint128::from(500u128),
            unstaking_period: 1,
        }
    );

    let msg = ExecuteMsg::WithdrawUnstaked {
        amount: Some(Uint128::from(800u128)),
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    assert_error(res, "Cannot Sub with 500 and 800");

    let msg = ExecuteMsg::WithdrawUnstaked {
        amount: None,
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: XASTRO_TOKEN.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: USER1.to_string(),
                amount: Uint128::from(500u128),
            })?,
            funds: vec![],
        }),]
    );

    let msg = QueryMsg::StakingState {};
    let res: StakingState = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        StakingState {
            total_bond_share: Uint128::from(200u128),
            reward_index: Decimal::from_ratio(11375u128, 100000u128),
            next_claim_period: 1,
            total_unstaking_amount: Uint128::from(300u128),
            total_unstaked_amount: Uint128::zero(),
            unstaking_period: 1,
        }
    );

    let msg = QueryMsg::StakerInfo {
        user: USER1.to_string(),
    };
    let res: StakerInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        StakerInfoResponse {
            bond_share: Uint128::zero(),
            bond_amount: Uint128::zero(),
            reward_index: Decimal::from_ratio(11375u128, 100000u128),
            pending_reward: Uint128::from(41u128),
            unstaking_amount: Uint128::from(300u128),
            unstaked_amount: Uint128::zero(),
            unstaking_period: 1,
        }
    );

    Ok(())
}

#[allow(clippy::redundant_clone)]
fn claim_income(
    deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
) -> Result<(), ContractError> {
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(EPOCH_START + WEEK);
    let info = mock_info(USER2, &[]);

    let msg = QueryMsg::StakerInfo {
        user: USER2.to_string(),
    };
    let res: StakerInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        StakerInfoResponse {
            bond_share: Uint128::from(200u128),
            bond_amount: Uint128::from(200u128),
            reward_index: Decimal::from_ratio(11375u128, 100000u128),
            pending_reward: Uint128::from(20u128),
            unstaking_amount: Uint128::zero(),
            unstaked_amount: Uint128::zero(),
            unstaking_period: 1,
        }
    );

    let msg = ExecuteMsg::ClaimIncome {};
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: ASTRO_TOKEN.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: USER2.to_string(),
                amount: Uint128::from(20u128),
            })?,
            funds: vec![],
        }),]
    );

    let msg = QueryMsg::RewardInfo {
        token: ASTRO_TOKEN.to_string(),
    };
    let res: RewardInfo = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        RewardInfo {
            staker_income: Uint128::zero(),
            fee: Uint128::from(5u128),
            reconciled_amount: Uint128::from(54u128),
        }
    );

    let msg = QueryMsg::StakerInfo {
        user: USER2.to_string(),
    };
    let res: StakerInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        StakerInfoResponse {
            bond_share: Uint128::from(200u128),
            bond_amount: Uint128::from(200u128),
            reward_index: Decimal::from_ratio(11375u128, 100000u128),
            pending_reward: Uint128::zero(),
            unstaking_amount: Uint128::zero(),
            unstaked_amount: Uint128::zero(),
            unstaking_period: 1,
        }
    );

    Ok(())
}

#[allow(clippy::redundant_clone)]
fn send_income(
    deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
) -> Result<(), ContractError> {
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(EPOCH_START + WEEK);
    let info = mock_info(USER2, &[]);

    let msg = ExecuteMsg::SendIncome {};
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert_error(res, "Unauthorized");

    let info = mock_info(CONTROLLER, &[]);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: ASTRO_TOKEN.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: FEE_COLLECTOR.to_string(),
                amount: Uint128::from(5u128),
            })?,
            funds: vec![],
        }),]
    );

    let msg = QueryMsg::RewardInfo {
        token: ASTRO_TOKEN.to_string(),
    };
    let res: RewardInfo = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        RewardInfo {
            staker_income: Uint128::zero(),
            fee: Uint128::zero(),
            reconciled_amount: Uint128::from(49u128),
        }
    );

    Ok(())
}
