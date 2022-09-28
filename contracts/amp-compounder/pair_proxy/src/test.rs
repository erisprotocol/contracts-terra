use astroport::asset::{Asset, AssetInfo, PairInfo};
use astroport::factory::PairType;
use astroport::router::{
    Cw20HookMsg as RouterCw20HookMsg, ExecuteMsg as RouterExecuteMsg, SwapOperation,
};
use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, Decimal, OwnedDeps, Response, StdError, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use eris::adapters::router::{Router, RouterType};
use eris::pair_proxy::{Cw20HookMsg, ExecuteMsg, InstantiateMsg};

use crate::contract::{execute, instantiate};
use crate::error::ContractError;
use crate::mock_querier::{mock_dependencies, WasmMockQuerier};
use crate::state::{Config, CONFIG};

const USER_1: &str = "user_1";
const USER_2: &str = "user_2";
const ROUTER: &str = "router";
const TOKEN_1: &str = "token_1";
const TOKEN_2: &str = "token_2";
const IBC_TOKEN: &str = "ibc/stablecoin";

#[test]
fn test() -> Result<(), ContractError> {
    let mut deps = mock_dependencies();
    create(&mut deps)?;
    swap(&mut deps)?;

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

fn create(
    deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
) -> Result<(), ContractError> {
    let env = mock_env();
    let info = mock_info(USER_1, &[]);

    let instantiate_msg = InstantiateMsg {
        asset_infos: vec![],
        router: ROUTER.to_string(),
        router_type: RouterType::AstroSwap,
        offer_precision: None,
        ask_precision: None,
    };
    let res = instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg);
    assert_error(res, "Must provide at least 2 assets!");

    let instantiate_msg = InstantiateMsg {
        asset_infos: vec![
            AssetInfo::Token {
                contract_addr: Addr::unchecked(TOKEN_1),
            },
            AssetInfo::Token {
                contract_addr: Addr::unchecked(TOKEN_1),
            },
        ],
        router: ROUTER.to_string(),
        router_type: RouterType::AstroSwap,
        offer_precision: None,
        ask_precision: None,
    };
    let res = instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg);
    assert_error(res, "Duplicated assets in asset infos");

    let instantiate_msg = InstantiateMsg {
        asset_infos: vec![
            AssetInfo::Token {
                contract_addr: Addr::unchecked(TOKEN_1),
            },
            AssetInfo::Token {
                contract_addr: Addr::unchecked(TOKEN_2),
            },
            AssetInfo::NativeToken {
                denom: IBC_TOKEN.to_string(),
            },
        ],
        router: ROUTER.to_string(),
        router_type: RouterType::AstroSwap,
        offer_precision: None,
        ask_precision: None,
    };
    let res = instantiate(deps.as_mut(), env, info, instantiate_msg);
    assert!(res.is_ok());

    let config = CONFIG.load(deps.as_mut().storage)?;
    assert_eq!(
        config,
        Config {
            pair_info: PairInfo {
                asset_infos: vec![
                    AssetInfo::Token {
                        contract_addr: Addr::unchecked(TOKEN_1),
                    },
                    AssetInfo::NativeToken {
                        denom: IBC_TOKEN.to_string(),
                    },
                ],
                contract_addr: Addr::unchecked(MOCK_CONTRACT_ADDR),
                liquidity_token: Addr::unchecked(""),
                pair_type: PairType::Custom("pair_proxy".to_string())
            },
            asset_infos: vec![
                AssetInfo::Token {
                    contract_addr: Addr::unchecked(TOKEN_1),
                },
                AssetInfo::Token {
                    contract_addr: Addr::unchecked(TOKEN_2),
                },
                AssetInfo::NativeToken {
                    denom: IBC_TOKEN.to_string(),
                },
            ],
            router: Router(Addr::unchecked(ROUTER)),
            router_type: RouterType::AstroSwap,
            offer_precision: 6,
            ask_precision: 6
        }
    );

    Ok(())
}

fn swap(deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>) -> Result<(), ContractError> {
    let env = mock_env();

    let info = mock_info(TOKEN_1, &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: USER_1.to_string(),
        amount: Uint128::from(100u128),
        msg: to_binary(&Cw20HookMsg::Swap {
            belief_price: Some(Decimal::percent(100)),
            max_spread: Some(Decimal::percent(1)),
            to: Some(USER_2.to_string()),
        })?,
    });

    let res = execute(deps.as_mut(), env.clone(), info, msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: TOKEN_1.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: ROUTER.to_string(),
                amount: Uint128::from(100u128),
                msg: to_binary(&RouterCw20HookMsg::ExecuteSwapOperations {
                    operations: vec![
                        SwapOperation::AstroSwap {
                            offer_asset_info: AssetInfo::Token {
                                contract_addr: Addr::unchecked(TOKEN_1.to_string())
                            },
                            ask_asset_info: AssetInfo::Token {
                                contract_addr: Addr::unchecked(TOKEN_2.to_string())
                            }
                        },
                        SwapOperation::AstroSwap {
                            offer_asset_info: AssetInfo::Token {
                                contract_addr: Addr::unchecked(TOKEN_2.to_string())
                            },
                            ask_asset_info: AssetInfo::NativeToken {
                                denom: IBC_TOKEN.to_string(),
                            },
                        },
                    ],
                    minimum_receive: Some(Uint128::from(99u128)),
                    to: Some(USER_2.to_string()),
                    max_spread: Some(Decimal::percent(1))
                })?,
            })?,
            funds: vec![],
        }),]
    );

    let info = mock_info(
        USER_1,
        &[Coin {
            denom: IBC_TOKEN.to_string(),
            amount: Uint128::from(100u128),
        }],
    );
    let msg = ExecuteMsg::Swap {
        offer_asset: Asset {
            info: AssetInfo::NativeToken {
                denom: IBC_TOKEN.to_string(),
            },
            amount: Uint128::from(100u128),
        },
        belief_price: Some(Decimal::percent(100)),
        max_spread: Some(Decimal::percent(1)),
        to: None,
    };

    let res = execute(deps.as_mut(), env, info, msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: ROUTER.to_string(),
            msg: to_binary(&RouterExecuteMsg::ExecuteSwapOperations {
                operations: vec![
                    SwapOperation::AstroSwap {
                        offer_asset_info: AssetInfo::NativeToken {
                            denom: IBC_TOKEN.to_string(),
                        },
                        ask_asset_info: AssetInfo::Token {
                            contract_addr: Addr::unchecked(TOKEN_2.to_string())
                        },
                    },
                    SwapOperation::AstroSwap {
                        offer_asset_info: AssetInfo::Token {
                            contract_addr: Addr::unchecked(TOKEN_2.to_string())
                        },
                        ask_asset_info: AssetInfo::Token {
                            contract_addr: Addr::unchecked(TOKEN_1.to_string())
                        }
                    },
                ],
                minimum_receive: Some(Uint128::from(99u128)),
                to: Some(USER_1.to_string()),
                max_spread: Some(Decimal::percent(1))
            })?,
            funds: vec![Coin {
                denom: IBC_TOKEN.to_string(),
                amount: Uint128::from(100u128),
            }],
        }),]
    );

    Ok(())
}
