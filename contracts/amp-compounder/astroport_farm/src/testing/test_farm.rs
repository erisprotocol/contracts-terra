use crate::contract::{execute, instantiate, query, reply};
use crate::error::ContractError;
use astroport::asset::{native_asset, token_asset, Asset, AssetInfo};
use astroport::generator::{
    Cw20HookMsg as GeneratorCw20HookMsg, ExecuteMsg as GeneratorExecuteMsg,
};

use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    attr, coin, from_binary, to_binary, Addr, Coin, CosmosMsg, Decimal, DepsMut, Env, Event,
    MessageInfo, OwnedDeps, Reply, Response, StdError, SubMsgResponse, Timestamp, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, Expiration};
use eris::adapters::token::Token;
use eris::astroport_farm::{
    CallbackMsg, ConfigResponse, Cw20HookMsg, ExchangeRatesResponse, ExecuteMsg, InstantiateMsg,
    QueryMsg, StateResponse, TokenInit, UserInfoResponse,
};
use eris::compound_proxy::ExecuteMsg as CompoundProxyExecuteMsg;
use eris::constants::DAY;

use super::mock_querier::{mock_dependencies, WasmMockQuerier};

const ASTRO_TOKEN: &str = "astro";
const REWARD_TOKEN: &str = "reward";
const OWNER: &str = "owner";
const USER_1: &str = "user_1";
const USER_2: &str = "user_2";
const USER_3: &str = "user_3";
const GENERATOR_PROXY: &str = "generator_proxy";
const COMPOUND_PROXY: &str = "compound_proxy";
const CONTROLLER: &str = "controller";
const FEE_COLLECTOR: &str = "fee_collector";
const COMPOUND_PROXY_2: &str = "compound_proxy_2";
const CONTROLLER_2: &str = "controller_2";
const FEE_COLLECTOR_2: &str = "fee_collector_2";
const LP_TOKEN: &str = "lp_token";
const AMP_LP_TOKEN: &str = "amp_lp_token";
const IBC_TOKEN: &str = "ibc/stablecoin";

#[allow(clippy::redundant_clone)]
#[test]
fn test() -> Result<(), ContractError> {
    let mut deps = mock_dependencies();

    create(&mut deps)?;
    config(&mut deps)?;
    owner(&mut deps)?;
    bond(&mut deps)?;
    // _deposit_time(&mut deps)?;
    compound(&mut deps, 700)?;
    callback(&mut deps)?;

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
    let env = mock_env();

    // invalid fee percentage
    let info = mock_info(USER_1, &[]);
    let msg = InstantiateMsg {
        owner: USER_1.to_string(),
        staking_contract: GENERATOR_PROXY.to_string(),
        compound_proxy: COMPOUND_PROXY.to_string(),
        controller: CONTROLLER.to_string(),
        fee: Decimal::percent(101),
        fee_collector: FEE_COLLECTOR.to_string(),
        liquidity_token: LP_TOKEN.to_string(),
        base_reward_token: ASTRO_TOKEN.to_string(),
        amp_lp: TokenInit {
            cw20_code_id: 69420,
            decimals: 6,
            name: "ampLP-AXL-LUNA".to_string(),
            symbol: "ampLP".to_string(),
        },
        deposit_profit_delay_s: 0,
    };
    let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg);
    assert_error(res, "fee must be 0 to 1");

    // valid init message
    let msg = InstantiateMsg {
        owner: USER_1.to_string(),
        staking_contract: GENERATOR_PROXY.to_string(),
        compound_proxy: COMPOUND_PROXY.to_string(),
        controller: CONTROLLER.to_string(),
        fee: Decimal::percent(5),
        fee_collector: FEE_COLLECTOR.to_string(),
        liquidity_token: LP_TOKEN.to_string(),
        base_reward_token: ASTRO_TOKEN.to_string(),
        amp_lp: TokenInit {
            cw20_code_id: 69420,
            decimals: 6,
            name: "ampLP-AXL-LUNA".to_string(),
            symbol: "ampLP".to_string(),
        },
        deposit_profit_delay_s: 0,
    };

    let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
    assert_eq!(res.messages.len(), 1);

    let event = Event::new("instantiate")
        .add_attribute("creator", MOCK_CONTRACT_ADDR)
        .add_attribute("admin", "admin")
        .add_attribute("code_id", "69420")
        .add_attribute("_contract_address", AMP_LP_TOKEN);

    let _res = reply(
        deps.as_mut(),
        env.clone(),
        Reply {
            id: 1,
            result: cosmwasm_std::SubMsgResult::Ok(SubMsgResponse {
                events: vec![event],
                data: None,
            }),
        },
    )
    .unwrap();

    // query config
    let msg = QueryMsg::Config {};
    let res: ConfigResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        ConfigResponse {
            owner: Addr::unchecked(USER_1),
            controller: Addr::unchecked(CONTROLLER),
            fee_collector: Addr::unchecked(FEE_COLLECTOR),
            staking_contract: Addr::unchecked(GENERATOR_PROXY),
            compound_proxy: Addr::unchecked(COMPOUND_PROXY),
            fee: Decimal::percent(5),
            lp_token: Addr::unchecked(LP_TOKEN.to_string()),
            base_reward_token: Addr::unchecked(ASTRO_TOKEN.to_string()),
            deposit_profit_delay_s: 0,
            amp_lp_token: Addr::unchecked(AMP_LP_TOKEN)
        }
    );

    Ok(())
}

#[allow(clippy::redundant_clone)]
fn config(
    deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
) -> Result<(), ContractError> {
    let env = mock_env();

    let info = mock_info(USER_2, &[]);
    let msg = ExecuteMsg::UpdateConfig {
        compound_proxy: None,
        controller: None,
        fee: Some(Decimal::percent(101)),
        fee_collector: None,
        deposit_profit_delay_s: None,
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert_error(res, "Unauthorized");

    let info = mock_info(USER_1, &[]);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    assert_error(res, "fee must be 0 to 1");

    let msg = ExecuteMsg::UpdateConfig {
        compound_proxy: None,
        controller: None,
        fee: Some(Decimal::percent(3)),
        fee_collector: None,
        deposit_profit_delay_s: None,
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert!(res.is_ok());

    let msg = ExecuteMsg::UpdateConfig {
        compound_proxy: Some(COMPOUND_PROXY_2.to_string()),
        controller: None,
        fee: None,
        fee_collector: None,
        deposit_profit_delay_s: None,
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert!(res.is_ok());

    let msg = ExecuteMsg::UpdateConfig {
        compound_proxy: None,
        controller: Some(CONTROLLER_2.to_string()),
        fee: None,
        fee_collector: None,
        deposit_profit_delay_s: None,
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert!(res.is_ok());

    let msg = ExecuteMsg::UpdateConfig {
        compound_proxy: None,
        controller: None,
        fee: None,
        fee_collector: Some(FEE_COLLECTOR_2.to_string()),
        deposit_profit_delay_s: None,
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert!(res.is_ok());

    let msg = QueryMsg::Config {};
    let res: ConfigResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        ConfigResponse {
            owner: Addr::unchecked(USER_1),
            controller: Addr::unchecked(CONTROLLER_2),
            fee_collector: Addr::unchecked(FEE_COLLECTOR_2),
            staking_contract: Addr::unchecked(GENERATOR_PROXY),
            compound_proxy: Addr::unchecked(COMPOUND_PROXY_2),
            fee: Decimal::percent(3),
            lp_token: Addr::unchecked(LP_TOKEN.to_string()),
            base_reward_token: Addr::unchecked(ASTRO_TOKEN.to_string()),
            deposit_profit_delay_s: 0,
            amp_lp_token: Addr::unchecked(AMP_LP_TOKEN)
        }
    );

    let msg = ExecuteMsg::UpdateConfig {
        compound_proxy: Some(COMPOUND_PROXY.to_string()),
        controller: Some(CONTROLLER.to_string()),
        fee: Some(Decimal::percent(5)),
        fee_collector: Some(FEE_COLLECTOR.to_string()),
        deposit_profit_delay_s: None,
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert!(res.is_ok());

    let msg = QueryMsg::Config {};
    let res: ConfigResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        ConfigResponse {
            owner: Addr::unchecked(USER_1),
            controller: Addr::unchecked(CONTROLLER),
            fee_collector: Addr::unchecked(FEE_COLLECTOR),
            staking_contract: Addr::unchecked(GENERATOR_PROXY),
            compound_proxy: Addr::unchecked(COMPOUND_PROXY),
            fee: Decimal::percent(5),
            lp_token: Addr::unchecked(LP_TOKEN.to_string()),
            base_reward_token: Addr::unchecked(ASTRO_TOKEN.to_string()),
            deposit_profit_delay_s: 0,
            amp_lp_token: Addr::unchecked(AMP_LP_TOKEN)
        }
    );

    Ok(())
}

#[allow(clippy::redundant_clone)]
fn owner(deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>) -> Result<(), ContractError> {
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(0);

    // new owner
    let msg = ExecuteMsg::ProposeNewOwner {
        owner: OWNER.to_string(),
        expires_in: 100,
    };

    let info = mock_info(USER_2, &[]);

    // unauthorized check
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert_error(res, "Unauthorized");

    // claim before a proposal
    let info = mock_info(USER_2, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, ExecuteMsg::ClaimOwnership {});
    assert_error(res, "Ownership proposal not found");

    // propose new owner
    let info = mock_info(USER_1, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert!(res.is_ok());

    // drop ownership proposal
    let info = mock_info(USER_1, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, ExecuteMsg::DropOwnershipProposal {});
    assert!(res.is_ok());

    // ownership proposal dropped
    let info = mock_info(USER_2, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, ExecuteMsg::ClaimOwnership {});
    assert_error(res, "Ownership proposal not found");

    // propose new owner again
    let info = mock_info(USER_1, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg);
    assert!(res.is_ok());

    // unauthorized ownership claim
    let info = mock_info(USER_3, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, ExecuteMsg::ClaimOwnership {});
    assert_error(res, "Unauthorized");

    env.block.time = Timestamp::from_seconds(101);

    // ownership proposal expired
    let info = mock_info(OWNER, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, ExecuteMsg::ClaimOwnership {});
    assert_error(res, "Ownership proposal expired");

    env.block.time = Timestamp::from_seconds(100);

    // claim ownership
    let info = mock_info(OWNER, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, ExecuteMsg::ClaimOwnership {})?;
    assert_eq!(0, res.messages.len());

    // query config
    let config: ConfigResponse =
        from_binary(&query(deps.as_ref(), env.clone(), QueryMsg::Config {})?)?;
    assert_eq!(OWNER, config.owner);
    Ok(())
}

#[allow(clippy::redundant_clone)]
fn bond(deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>) -> Result<(), ContractError> {
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(101);

    // invalid staking token
    let info = mock_info(ASTRO_TOKEN, &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: USER_1.to_string(),
        amount: Uint128::from(100000u128),
        msg: to_binary(&Cw20HookMsg::Bond {
            staker_addr: None,
        })?,
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
    assert_error(res, "Unauthorized");

    // user_1 bond 100000 LP
    let info = mock_info(LP_TOKEN, &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: USER_1.to_string(),
        amount: Uint128::from(100000u128),
        msg: to_binary(&Cw20HookMsg::Bond {
            staker_addr: None,
        })?,
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg)?;

    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [
            mint_and_msg(deps, AMP_LP_TOKEN, USER_1, 100000),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: LP_TOKEN.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: GENERATOR_PROXY.to_string(),
                    amount: Uint128::from(100000u128),
                    msg: to_binary(&GeneratorCw20HookMsg::Deposit {})?,
                })?,
                funds: vec![],
            }),
        ]
    );

    // update generator balance
    deps.querier.set_balance(GENERATOR_PROXY, LP_TOKEN, 100000);

    // query reward info
    let msg = QueryMsg::UserInfo {
        addr: USER_1.to_string(),
    };
    let res: UserInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        UserInfoResponse {
            user_lp_amount: Uint128::from(100000u128),
            user_amp_lp_amount: Uint128::from(100000u128),
            total_lp: Uint128::from(100000u128),
            total_amp_lp: Uint128::from(100000u128),
        }
    );

    // update time
    env.block.time = Timestamp::from_seconds(102);

    // user_1 bond 100000 LP for user_2
    let info = mock_info(LP_TOKEN, &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: USER_1.to_string(),
        amount: Uint128::from(50000u128),
        msg: to_binary(&Cw20HookMsg::Bond {
            staker_addr: Some(USER_2.to_string()),
        })?,
    });
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [
            mint_and_msg(deps, AMP_LP_TOKEN, USER_2, 50000),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: LP_TOKEN.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: GENERATOR_PROXY.to_string(),
                    amount: Uint128::from(50000u128),
                    msg: to_binary(&GeneratorCw20HookMsg::Deposit {})?,
                })?,
                funds: vec![],
            }),
        ]
    );

    // update generator balance
    env.block.time = Timestamp::from_seconds(100102);
    deps.querier.set_balance(GENERATOR_PROXY, LP_TOKEN, 150000);

    // query reward info
    let msg = QueryMsg::UserInfo {
        addr: USER_2.to_string(),
    };
    let res: UserInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        UserInfoResponse {
            user_lp_amount: Uint128::from(50000u128),
            user_amp_lp_amount: Uint128::from(50000u128),
            total_lp: Uint128::from(150000u128),
            total_amp_lp: Uint128::from(150000u128),
        }
    );

    // query state
    let msg = QueryMsg::State {
        addr: None,
    };
    let res: StateResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        StateResponse {
            total_lp: Uint128::from(150000u128),
            total_amp_lp: Uint128::from(150000u128),
            user_info: None,
            exchange_rate: Decimal::one(),
            locked_assets: vec![
                native_asset("asset1".to_string(), Uint128::new(15000)),
                token_asset(Addr::unchecked("asset2"), Uint128::new(30000))
            ],
            pair_contract: Addr::unchecked("pair")
        }
    );

    // increase generator balance by 30000 from compound
    deps.querier.set_balance(GENERATOR_PROXY, LP_TOKEN, 180000);

    // query reward info for user_1, bond amount should be 100000 + 20000 = 120000
    let msg = QueryMsg::UserInfo {
        addr: USER_1.to_string(),
    };
    let res: UserInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        UserInfoResponse {
            user_lp_amount: Uint128::from(120000u128),
            user_amp_lp_amount: Uint128::from(100000u128),
            total_lp: Uint128::from(180000u128),
            total_amp_lp: Uint128::from(150000u128),
        }
    );

    // query reward info for user_2, bond amount should be 50000 + 10000 = 60000
    let msg = QueryMsg::UserInfo {
        addr: USER_2.to_string(),
    };
    let res: UserInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        UserInfoResponse {
            user_lp_amount: Uint128::from(60000u128),
            user_amp_lp_amount: Uint128::from(50000u128),
            total_lp: Uint128::from(180000u128),
            total_amp_lp: Uint128::from(150000u128),
        }
    );

    // unbond for user_1
    let info = mock_info(USER_1, &[]);

    let res =
        execute_unbond_msg(deps.as_mut(), env.clone(), info.clone(), Uint128::from(50000u128))?;

    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [
            burn_and_msg(deps, AMP_LP_TOKEN, USER_1, 50000),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: GENERATOR_PROXY.to_string(),
                msg: to_binary(&GeneratorExecuteMsg::Withdraw {
                    lp_token: LP_TOKEN.to_string(),
                    amount: Uint128::from(60000u128)
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: LP_TOKEN.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: USER_1.to_string(),
                    amount: Uint128::from(60000u128)
                })?,
                funds: vec![],
            }),
        ]
    );

    // decrease generator balance by 60000 from withdrawal
    deps.querier.set_balance(GENERATOR_PROXY, LP_TOKEN, 120000);

    // query reward info for user_1, bond amount should be 120000 - 60000 = 60000
    let msg = QueryMsg::UserInfo {
        addr: USER_1.to_string(),
    };
    let res: UserInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        UserInfoResponse {
            user_lp_amount: Uint128::from(60000u128),
            user_amp_lp_amount: Uint128::from(50000u128),
            total_lp: Uint128::from(120000u128),
            total_amp_lp: Uint128::from(100000u128),
        }
    );

    // query reward info for user_2
    let msg = QueryMsg::UserInfo {
        addr: USER_2.to_string(),
    };
    let res: UserInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        UserInfoResponse {
            user_lp_amount: Uint128::from(60000u128),
            user_amp_lp_amount: Uint128::from(50000u128),
            total_lp: Uint128::from(120000u128),
            total_amp_lp: Uint128::from(100000u128),
        }
    );

    // unbond for user_2
    let info = mock_info(USER_2, &[]);
    let res =
        execute_unbond_msg(deps.as_mut(), env.clone(), info.clone(), Uint128::from(50000u128))?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [
            burn_and_msg(deps, AMP_LP_TOKEN, USER_2, 50000),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: GENERATOR_PROXY.to_string(),
                msg: to_binary(&GeneratorExecuteMsg::Withdraw {
                    lp_token: LP_TOKEN.to_string(),
                    amount: Uint128::from(60000u128)
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: LP_TOKEN.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: USER_2.to_string(),
                    amount: Uint128::from(60000u128)
                })?,
                funds: vec![],
            }),
        ]
    );

    // decrease generator balance by 60000 from withdrawal
    deps.querier.set_balance(GENERATOR_PROXY, LP_TOKEN, 60000);

    // query reward info for user_2, bond amount should be 60000 - 60000 = 0
    let msg = QueryMsg::UserInfo {
        addr: USER_2.to_string(),
    };
    let res: UserInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        UserInfoResponse {
            user_lp_amount: Uint128::from(0u128),
            user_amp_lp_amount: Uint128::from(0u128),
            total_lp: Uint128::from(60000u128),
            total_amp_lp: Uint128::from(50000u128),
        }
    );

    // query reward info for user_1
    let msg = QueryMsg::UserInfo {
        addr: USER_1.to_string(),
    };
    let res: UserInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        UserInfoResponse {
            user_lp_amount: Uint128::from(60000u128),
            user_amp_lp_amount: Uint128::from(50000u128),
            total_lp: Uint128::from(60000u128),
            total_amp_lp: Uint128::from(50000u128),
        }
    );

    // update time
    env.block.height = 600;

    // set LP token balance of the contract
    deps.querier.set_cw20_balance(LP_TOKEN, MOCK_CONTRACT_ADDR, 142);

    // deposit assets for user_1
    let info = mock_info(USER_1, &[]);
    let assets = vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked(REWARD_TOKEN),
            },
            amount: Uint128::from(20000u128),
        },
        Asset {
            info: AssetInfo::NativeToken {
                denom: IBC_TOKEN.to_string(),
            },
            amount: Uint128::from(40000u128),
        },
    ];
    let msg = ExecuteMsg::BondAssets {
        assets: assets.clone(),
        minimum_receive: Some(Uint128::from(10000u128)),
        no_swap: None,
        receiver: None,
        slippage_tolerance: Some(Decimal::percent(2)),
    };

    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert_error(res, "Native token balance mismatch between the argument and the transferred");

    let info = mock_info(
        USER_1,
        &[Coin {
            denom: IBC_TOKEN.to_string(),
            amount: Uint128::from(40000u128),
        }],
    );
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: REWARD_TOKEN.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: USER_1.to_string(),
                    recipient: MOCK_CONTRACT_ADDR.to_string(),
                    amount: Uint128::from(20000u128)
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: REWARD_TOKEN.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: COMPOUND_PROXY.to_string(),
                    amount: Uint128::from(20000u128),
                    expires: Some(Expiration::AtHeight(601))
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: COMPOUND_PROXY.to_string(),
                msg: to_binary(&CompoundProxyExecuteMsg::Compound {
                    lp_token: LP_TOKEN.to_string(),
                    rewards: assets.clone(),
                    receiver: None,
                    no_swap: None,
                    slippage_tolerance: Some(Decimal::percent(2)),
                })?,
                funds: vec![Coin {
                    denom: IBC_TOKEN.to_string(),
                    amount: Uint128::from(40000u128),
                }],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::BondTo {
                    to: Addr::unchecked(USER_1),
                    prev_balance: Uint128::from(142u128),
                    minimum_receive: Some(Uint128::from(10000u128)),
                }))?,
                funds: vec![],
            }),
        ]
    );

    let msg = ExecuteMsg::BondAssets {
        assets: assets.clone(),
        minimum_receive: Some(Uint128::from(10000u128)),
        no_swap: Some(true),
        receiver: None,
        slippage_tolerance: None,
    };

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: REWARD_TOKEN.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: USER_1.to_string(),
                    recipient: MOCK_CONTRACT_ADDR.to_string(),
                    amount: Uint128::from(20000u128)
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: REWARD_TOKEN.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: COMPOUND_PROXY.to_string(),
                    amount: Uint128::from(20000u128),
                    expires: Some(Expiration::AtHeight(601))
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: COMPOUND_PROXY.to_string(),
                msg: to_binary(&CompoundProxyExecuteMsg::Compound {
                    lp_token: LP_TOKEN.to_string(),
                    rewards: assets,
                    receiver: None,
                    no_swap: Some(true),
                    slippage_tolerance: None,
                })?,
                funds: vec![Coin {
                    denom: IBC_TOKEN.to_string(),
                    amount: Uint128::from(40000u128),
                }],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::BondTo {
                    to: Addr::unchecked(USER_1),
                    prev_balance: Uint128::from(142u128),
                    minimum_receive: Some(Uint128::from(10000u128)),
                }))?,
                funds: vec![],
            }),
        ]
    );

    // update time
    env.block.time = Timestamp::from_seconds(200201);

    // set LP token balance of the contract
    deps.querier.set_cw20_balance(LP_TOKEN, MOCK_CONTRACT_ADDR, 10141);

    let msg = ExecuteMsg::Callback(CallbackMsg::BondTo {
        to: Addr::unchecked(USER_1),
        prev_balance: Uint128::from(142u128),
        minimum_receive: Some(Uint128::from(10000u128)),
    });
    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    // received less LP token than minimum_receive, received 10141 - 142 = 9999 LP
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert_error(res, "Assertion failed; minimum receive amount: 10000, actual amount: 9999");

    deps.querier.set_cw20_balance(LP_TOKEN, MOCK_CONTRACT_ADDR, 10142);
    let res = execute(deps.as_mut(), env.clone(), info, msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [
            mint_and_msg(deps, AMP_LP_TOKEN, USER_1, 8333),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: LP_TOKEN.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: GENERATOR_PROXY.to_string(),
                    amount: Uint128::from(10000u128),
                    msg: to_binary(&GeneratorCw20HookMsg::Deposit {})?,
                })?,
                funds: vec![],
            }),
        ]
    );

    deps.querier.set_cw20_balance(LP_TOKEN, MOCK_CONTRACT_ADDR, 142);

    // increase generator balance by 10000
    deps.querier.set_balance(GENERATOR_PROXY, LP_TOKEN, 70000u128);

    // query reward info for user_1, bond amount should be 60000 + 10000 = 70000
    let msg = QueryMsg::UserInfo {
        addr: USER_1.to_string(),
    };
    let res: UserInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        UserInfoResponse {
            user_lp_amount: Uint128::from(70000u128),
            user_amp_lp_amount: Uint128::from(58333u128),
            total_lp: Uint128::from(70000u128),
            total_amp_lp: Uint128::from(58333u128),
        }
    );

    // query state
    let msg = QueryMsg::State {
        addr: None,
    };
    let res: StateResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        StateResponse {
            total_lp: Uint128::from(70000u128),
            total_amp_lp: Uint128::from(58333u128),
            exchange_rate: Decimal::from_ratio(70000u128, 58333u128),
            locked_assets: vec![
                native_asset("asset1".to_string(), Uint128::new(7000)),
                token_asset(Addr::unchecked("asset2"), Uint128::new(14000))
            ],
            pair_contract: Addr::unchecked("pair"),
            user_info: None
        }
    );

    Ok(())
}

#[allow(clippy::redundant_clone)]
#[test]
fn bond_same_assets() -> Result<(), ContractError> {
    let mut deps = mock_dependencies();
    let env = mock_env();

    create(&mut deps)?;

    let info = mock_info(USER_1, &[coin(200, "uluna")]);

    // Check with native assets
    let assets = vec![
        native_asset("uluna".to_string(), Uint128::new(100)),
        native_asset("uluna".to_string(), Uint128::new(100)),
    ];

    let msg = ExecuteMsg::BondAssets {
        assets: assets.clone(),
        minimum_receive: Some(Uint128::from(10000u128)),
        no_swap: None,
        receiver: None,
        slippage_tolerance: Some(Decimal::percent(2)),
    };

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap_err();
    assert_eq!(res.to_string(), "Generic error: duplicated asset");

    // Check with tokens
    let assets = vec![
        token_asset(Addr::unchecked("token1"), Uint128::new(100)),
        token_asset(Addr::unchecked("token1"), Uint128::new(100)),
    ];
    let msg = ExecuteMsg::BondAssets {
        assets: assets.clone(),
        minimum_receive: Some(Uint128::from(10000u128)),
        no_swap: None,
        receiver: None,
        slippage_tolerance: Some(Decimal::percent(2)),
    };
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert_error(res, "duplicated asset");

    Ok(())
}

#[allow(clippy::redundant_clone)]
#[test]
fn bond_delayed_profit() -> Result<(), ContractError> {
    let mut deps = mock_dependencies();

    create(&mut deps)?;
    owner(&mut deps)?;
    bond(&mut deps)?;

    compound(&mut deps, 0)?;
    compound(&mut deps, 300)?;
    compound(&mut deps, DAY)?;

    let exchange_rates: ExchangeRatesResponse = from_binary(&query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::ExchangeRates {
            start_after: None,
            limit: None,
        },
    )?)?;
    assert_eq!(
        exchange_rates
            .exchange_rates
            .into_iter()
            .map(|a| format!("{0};{1}", a.0, a.1))
            .collect::<Vec<String>>(),
        vec![
            "86400;2.73772992988531363".to_string(),
            "300;2.2251555723175561".to_string(),
            "0;1.71258121474979857".to_string()
        ]
    );

    // 0.59 means 59% per day (1.71258121474979857 -> 2.73772992988531363)
    assert_eq!(exchange_rates.apr.map(|a| a.to_string()), Some("0.598598598598598598".to_string()));

    // query single value
    let exchange_rates: ExchangeRatesResponse = from_binary(&query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::ExchangeRates {
            start_after: Some(86400),
            limit: Some(1),
        },
    )?)?;
    assert_eq!(
        exchange_rates
            .exchange_rates
            .into_iter()
            .map(|a| format!("{0};{1}", a.0, a.1))
            .collect::<Vec<String>>(),
        vec!["300;2.2251555723175561".to_string()]
    );
    // cant calculate with a single value
    assert_eq!(exchange_rates.apr, None);

    // set deposit_profit_delay
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(OWNER, &[]),
        ExecuteMsg::UpdateConfig {
            compound_proxy: None,
            controller: None,
            fee: None,
            fee_collector: None,
            deposit_profit_delay_s: Some(DAY),
        },
    )
    .unwrap();

    // SKIPPING BOND ASSETS PART as share is calculated within the callback

    deps.querier.set_cw20_balance(LP_TOKEN, MOCK_CONTRACT_ADDR, 10142);
    let msg = ExecuteMsg::Callback(CallbackMsg::BondTo {
        to: Addr::unchecked(USER_1),
        prev_balance: Uint128::from(142u128),
        minimum_receive: Some(Uint128::from(10000u128)),
    });
    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env().clone(), info, msg)?;

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "ampf/bond"),
            attr("amount", "10000"),
            attr("bond_amount", "10000"),
            attr("bond_share_adjusted", "2284"),
            // the normal amount is 3652 without the change in deposit_profit_delay_s
            // which is ~59% higher than what is received
            attr("bond_share", "3652")
        ]
    );

    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [
            mint_and_msg(&mut deps, AMP_LP_TOKEN, USER_1, 2284),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: LP_TOKEN.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: GENERATOR_PROXY.to_string(),
                    amount: Uint128::from(10000u128),
                    msg: to_binary(&GeneratorCw20HookMsg::Deposit {})?,
                })?,
                funds: vec![],
            }),
        ]
    );

    Ok(())
}

#[allow(clippy::redundant_clone)]
fn mint_and_msg(
    deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
    token: &str,
    user: &str,
    amount: u128,
) -> CosmosMsg {
    let balance = deps.querier.get_cw20_balance(token, user);
    let supply = deps.querier.get_cw20_total_supply(token);
    deps.querier.set_cw20_balance(token, user, balance + amount);
    deps.querier.set_cw20_total_supply(token, supply + amount);
    Token(Addr::unchecked(token)).mint(Uint128::new(amount), Addr::unchecked(user)).unwrap()
}

fn burn_and_msg(
    deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
    token: &str,
    user: &str,
    amount: u128,
) -> CosmosMsg {
    let balance = deps.querier.get_cw20_balance(token, user);
    let supply = deps.querier.get_cw20_total_supply(token);
    deps.querier.set_cw20_balance(token, user, balance - amount);
    deps.querier.set_cw20_total_supply(token, supply - amount);
    Token(Addr::unchecked(token)).burn(Uint128::new(amount)).unwrap()
}

fn execute_unbond_msg(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    execute(deps, env, mock_info(AMP_LP_TOKEN, &[]), unbond_msg(amount, info.sender.to_string()))
}

fn unbond_msg(amount: Uint128, sender: String) -> ExecuteMsg {
    ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
        sender,
        amount,
        msg: to_binary(&Cw20HookMsg::Unbond {
            receiver: None,
        })
        .unwrap(),
    })
}

#[allow(clippy::redundant_clone)]
fn _deposit_time(
    deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
) -> Result<(), ContractError> {
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(300000);

    // user_3 bond 10000 LP
    let info = mock_info(LP_TOKEN, &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: USER_3.to_string(),
        amount: Uint128::from(10000u128),
        msg: to_binary(&Cw20HookMsg::Bond {
            staker_addr: None,
        })?,
    });
    let res = execute(deps.as_mut(), env.clone(), info, msg);
    assert!(res.is_ok());

    assert_eq!(res.unwrap().messages[0].msg, mint_and_msg(deps, AMP_LP_TOKEN, USER_3, 8333));

    // increase generator balance by 10000 + 5000 (from compound)
    deps.querier.set_balance(GENERATOR_PROXY, LP_TOKEN, 85000);

    // query reward info for user_3, should get only 10000
    let msg = QueryMsg::UserInfo {
        addr: USER_3.to_string(),
    };
    let res: UserInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        UserInfoResponse {
            user_lp_amount: Uint128::from(9999u128),
            user_amp_lp_amount: Uint128::from(8333u128),
            total_lp: Uint128::from(85000u128),
            total_amp_lp: Uint128::from(66666u128),
        }
    );

    env.block.time = Timestamp::from_seconds(343200);

    // query reward info for user_3, should increase to 10312 instead of 10624
    let msg = QueryMsg::UserInfo {
        addr: USER_3.to_string(),
    };
    let res: UserInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        UserInfoResponse {
            user_lp_amount: Uint128::from(10311u128),
            user_amp_lp_amount: Uint128::from(8333u128),
            total_lp: Uint128::zero(),
            total_amp_lp: Uint128::zero(),
        }
    );

    // query reward info for user_1, should be 74375
    let msg = QueryMsg::UserInfo {
        addr: USER_1.to_string(),
    };
    let res: UserInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        UserInfoResponse {
            user_lp_amount: Uint128::from(74375u128),
            user_amp_lp_amount: Uint128::from(58333u128),
            total_lp: Uint128::zero(),
            total_amp_lp: Uint128::zero(),
        }
    );

    // minimum time reached
    env.block.time = Timestamp::from_seconds(386400);

    // query reward info for user_3, should increase 10624
    let msg = QueryMsg::UserInfo {
        addr: USER_3.to_string(),
    };
    let res: UserInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        UserInfoResponse {
            user_lp_amount: Uint128::from(10624u128),
            user_amp_lp_amount: Uint128::from(8333u128),
            total_lp: Uint128::zero(),
            total_amp_lp: Uint128::zero(),
        }
    );

    // rewind time
    env.block.time = Timestamp::from_seconds(343200);

    // unbond for user_3
    let info = mock_info(USER_3, &[]);
    let res =
        execute_unbond_msg(deps.as_mut(), env.clone(), info.clone(), Uint128::from(10311u128))?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: GENERATOR_PROXY.to_string(),
                msg: to_binary(&GeneratorExecuteMsg::Withdraw {
                    lp_token: LP_TOKEN.to_string(),
                    amount: Uint128::from(10311u128)
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: LP_TOKEN.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: USER_3.to_string(),
                    amount: Uint128::from(10311u128)
                })?,
                funds: vec![],
            }),
        ]
    );

    // increase generator balance by 10311
    deps.querier.set_balance(GENERATOR_PROXY, LP_TOKEN, 74689);

    // query reward info for user_1, should be 74375 + 312 (from user_3 penalty)= 74687
    let msg = QueryMsg::UserInfo {
        addr: USER_1.to_string(),
    };
    let res: UserInfoResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        UserInfoResponse {
            user_lp_amount: Uint128::from(74689u128),
            user_amp_lp_amount: Uint128::from(58333u128),
            total_lp: Uint128::zero(),
            total_amp_lp: Uint128::zero(),
        }
    );

    Ok(())
}

#[allow(clippy::redundant_clone)]
fn compound(
    deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
    seconds: u64,
) -> Result<(), ContractError> {
    let mut env = mock_env();

    // reset LP token balance of the contract
    deps.querier.set_cw20_balance(LP_TOKEN, MOCK_CONTRACT_ADDR, 1);

    // set pending tokens
    deps.querier.set_generator_pending(ASTRO_TOKEN, GENERATOR_PROXY, 10000);
    deps.querier.set_generator_pending(REWARD_TOKEN, GENERATOR_PROXY, 50000);

    // set block height
    env.block.height = 700;
    env.block.time = Timestamp::from_seconds(seconds);

    // only controller can execute compound
    let info = mock_info(USER_1, &[]);
    let msg = ExecuteMsg::Compound {
        minimum_receive: Some(Uint128::from(29900u128)),
        slippage_tolerance: Some(Decimal::percent(3)),
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert_error(res, "Unauthorized");

    let info = mock_info(CONTROLLER, &[]);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: GENERATOR_PROXY.to_string(),
                msg: to_binary(&GeneratorExecuteMsg::ClaimRewards {
                    lp_tokens: vec![LP_TOKEN.to_string()]
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: ASTRO_TOKEN.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: COMPOUND_PROXY.to_string(),
                    amount: Uint128::from(9500u128),
                    expires: Some(Expiration::AtHeight(701))
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: ASTRO_TOKEN.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: FEE_COLLECTOR.to_string(),
                    amount: Uint128::from(500u128)
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: REWARD_TOKEN.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: COMPOUND_PROXY.to_string(),
                    amount: Uint128::from(47500u128),
                    expires: Some(Expiration::AtHeight(701))
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: REWARD_TOKEN.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: FEE_COLLECTOR.to_string(),
                    amount: Uint128::from(2500u128)
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: COMPOUND_PROXY.to_string(),
                msg: to_binary(&CompoundProxyExecuteMsg::Compound {
                    lp_token: LP_TOKEN.to_string(),
                    rewards: vec![
                        Asset {
                            info: AssetInfo::Token {
                                contract_addr: Addr::unchecked(ASTRO_TOKEN),
                            },
                            amount: Uint128::from(9500u128),
                        },
                        Asset {
                            info: AssetInfo::Token {
                                contract_addr: Addr::unchecked(REWARD_TOKEN),
                            },
                            amount: Uint128::from(47500u128),
                        },
                    ],
                    receiver: None,
                    no_swap: None,
                    slippage_tolerance: Some(Decimal::percent(3)),
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::Stake {
                    prev_balance: Uint128::from(1u128),
                    minimum_receive: Some(Uint128::from(29900u128)),
                }))?,
                funds: vec![],
            }),
        ]
    );

    // receive 29899 LP token from compound proxy
    deps.querier.set_cw20_balance(LP_TOKEN, MOCK_CONTRACT_ADDR, 29900);

    let msg = ExecuteMsg::Callback(CallbackMsg::Stake {
        prev_balance: Uint128::from(1u128),
        minimum_receive: Some(Uint128::from(29900u128)),
    });
    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);

    // received less LP token than minimum_receive, received 29900 - 1 = 29899 LP
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert_error(res, "Assertion failed; minimum receive amount: 29900, actual amount: 29899");

    deps.querier.set_cw20_balance(LP_TOKEN, MOCK_CONTRACT_ADDR, 29901);
    let res = execute(deps.as_mut(), env.clone(), info, msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: LP_TOKEN.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: GENERATOR_PROXY.to_string(),
                amount: Uint128::from(29900u128),
                msg: to_binary(&GeneratorCw20HookMsg::Deposit {})?,
            })?,
            funds: vec![],
        }),]
    );

    // updates exchange rate (reversed for Deposit query)
    let balance = deps.querier.get_balance(GENERATOR_PROXY.to_string(), LP_TOKEN.to_string());
    deps.querier.set_balance(GENERATOR_PROXY, LP_TOKEN, balance.u128() + 29900);

    Ok(())
}

#[allow(clippy::redundant_clone)]
fn callback(
    deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
) -> Result<(), ContractError> {
    let env = mock_env();

    let msg = ExecuteMsg::Callback(CallbackMsg::Stake {
        prev_balance: Uint128::zero(),
        minimum_receive: None,
    });

    let info = mock_info(USER_1, &[]);

    // only contract itself can execute callback
    let res = execute(deps.as_mut(), env.clone(), info, msg);
    assert_error(res, "Unauthorized");

    let msg = ExecuteMsg::Callback(CallbackMsg::BondTo {
        to: Addr::unchecked(USER_1),
        prev_balance: Uint128::zero(),
        minimum_receive: None,
    });
    let info = mock_info(USER_1, &[]);

    // only contract itself can execute callback
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert_error(res, "Unauthorized");

    Ok(())
}
