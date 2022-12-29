use astroport::asset::{
    native_asset, native_asset_info, token_asset, token_asset_info, Asset, AssetInfo, PairInfo,
};
use astroport::pair::{
    Cw20HookMsg as AstroportPairCw20HookMsg, ExecuteMsg as AstroportPairExecuteMsg,
};
use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    coin, from_binary, to_binary, Addr, Coin, CosmosMsg, Decimal, StdError, StdResult, Uint128,
    Uint256, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use eris::adapters::asset::AssetEx;
use eris::adapters::pair::Pair;
use eris::compound_proxy::{
    CallbackMsg, CompoundSimulationResponse, ExecuteMsg, InstantiateMsg, LpConfig, LpInit,
    QueryMsg, RouteDelete, RouteInit, RouteResponseItem, RouteTypeResponseItem,
};
use proptest::std_facade::vec;

use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::execute::get_swap_amount;
use crate::state::{Config, RouteConfig, RouteType, State};
use crate::testing::mock_querier::mock_dependencies;
use eris::adapters::factory::Factory;
use eris::adapters::router::Router;
use eris::adapters::router::RouterType;

#[allow(clippy::redundant_clone)]
fn init_contract(
    pair_contract: Option<String>,
    wanted_token: Option<AssetInfo>,
) -> cosmwasm_std::OwnedDeps<
    cosmwasm_std::MemoryStorage,
    cosmwasm_std::testing::MockApi,
    super::mock_querier::WasmMockQuerier,
> {
    let mut deps = mock_dependencies(&[]);
    let msg = InstantiateMsg {
        lps: vec![
            LpInit {
                slippage_tolerance: Decimal::percent(1),
                pair_contract: pair_contract.unwrap_or_else(|| "pair_contract".to_string()),
                commission_bps: 30,
                wanted_token: wanted_token.unwrap_or_else(uluna),
            },
            LpInit {
                slippage_tolerance: Decimal::percent(1),
                pair_contract: "pair_astro_token".to_string(),
                commission_bps: 30,
                wanted_token: astro(),
            },
        ],
        routes: vec![
            RouteInit::Path {
                router: "router".to_string(),
                router_type: RouterType::AstroSwap,
                route: vec![astro(), any(), uluna()],
            },
            // any->uluna, uluna->any
            RouteInit::PairProxy {
                single_direction_from: None,
                pair_contract: "pair0001".to_string(),
            },
            // ibc->uluna
            RouteInit::PairProxy {
                single_direction_from: Some(ibc()),
                pair_contract: "pair0002".to_string(),
            },
            // astro->token, token->astro
            RouteInit::PairProxy {
                single_direction_from: None,
                pair_contract: "pair_astro_token".to_string(),
            },
        ],
        factory: Some("factory".to_string()),
        owner: "owner".to_string(),
    };
    let sender = "addr0000";
    let env = mock_env();
    let info = mock_info(sender, &[]);
    let res = instantiate(deps.as_mut(), env.clone(), info, msg);
    assert!(res.is_ok());
    deps
}

#[allow(clippy::redundant_clone)]
#[test]
fn proper_initialization() -> StdResult<()> {
    let deps = init_contract(None, None);
    let env = mock_env();

    let msg = QueryMsg::Config {};
    let config: Config = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;

    assert_eq!(
        config,
        Config {
            factory: Some(Factory(Addr::unchecked("factory"))),
            owner: Addr::unchecked("owner")
        }
    );

    let msg = QueryMsg::GetLp {
        lp_addr: "liquidity_token".to_string(),
    };
    let lp_config: LpConfig = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;

    assert_eq!(
        lp_config.pair_info,
        PairInfo {
            asset_infos: vec![token(), uluna()],
            contract_addr: Addr::unchecked("pair_contract"),
            liquidity_token: Addr::unchecked("liquidity_token"),
            pair_type: astroport::factory::PairType::Xyk {}
        }
    );

    let msg = QueryMsg::GetLps {
        start_after: None,
        limit: None,
    };
    let lp_configs: Vec<LpConfig> = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;

    assert_eq!(
        lp_configs,
        vec![
            LpConfig {
                pair_info: PairInfo {
                    asset_infos: vec![astro(), token()],
                    contract_addr: Addr::unchecked("pair_astro_token"),
                    liquidity_token: Addr::unchecked("astro_token_lp"),
                    pair_type: astroport::factory::PairType::Xyk {}
                },
                commission_bps: 30,
                slippage_tolerance: Decimal::percent(1),
                wanted_token: astro()
            },
            LpConfig {
                pair_info: PairInfo {
                    asset_infos: vec![token(), uluna()],
                    contract_addr: Addr::unchecked("pair_contract"),
                    liquidity_token: Addr::unchecked("liquidity_token"),
                    pair_type: astroport::factory::PairType::Xyk {}
                },
                commission_bps: 30,
                slippage_tolerance: Decimal::percent(1),
                wanted_token: uluna()
            }
        ]
    );

    let msg = QueryMsg::GetRoutes {
        start_after: None,
        limit: None,
    };
    let routes: Vec<RouteResponseItem> = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;

    assert_eq!(
        routes,
        vec![
            RouteResponseItem {
                key: ("any".to_string(), "uluna".to_string()),
                route_type: RouteTypeResponseItem::PairProxy {
                    pair_contract: "pair0001".to_string(),
                    asset_infos: vec!["any".to_string(), "uluna".to_string()]
                }
            },
            RouteResponseItem {
                key: ("astro".to_string(), "token".to_string()),
                route_type: RouteTypeResponseItem::PairProxy {
                    pair_contract: "pair_astro_token".to_string(),
                    asset_infos: vec!["astro".to_string(), "token".to_string()]
                }
            },
            RouteResponseItem {
                key: ("astro".to_string(), "uluna".to_string()),
                route_type: RouteTypeResponseItem::Path {
                    router: "router".to_string(),
                    router_type: RouterType::AstroSwap,
                    route: vec!["astro".to_string(), "any".to_string(), "uluna".to_string()]
                }
            },
            RouteResponseItem {
                key: ("token".to_string(), "astro".to_string()),
                route_type: RouteTypeResponseItem::PairProxy {
                    pair_contract: "pair_astro_token".to_string(),
                    asset_infos: vec!["astro".to_string(), "token".to_string()]
                }
            },
            RouteResponseItem {
                key: ("uluna".to_string(), "any".to_string()),
                route_type: RouteTypeResponseItem::PairProxy {
                    pair_contract: "pair0001".to_string(),
                    asset_infos: vec!["any".to_string(), "uluna".to_string()]
                }
            },
            RouteResponseItem {
                key: ("uluna".to_string(), "astro".to_string()),
                route_type: RouteTypeResponseItem::Path {
                    router: "router".to_string(),
                    router_type: RouterType::AstroSwap,
                    route: vec!["uluna".to_string(), "any".to_string(), "astro".to_string()]
                }
            },
            RouteResponseItem {
                key: ("ibc/token".to_string(), "uluna".to_string()),
                route_type: RouteTypeResponseItem::PairProxy {
                    pair_contract: "pair0002".to_string(),
                    asset_infos: vec!["uluna".to_string(), "ibc/token".to_string()]
                }
            }
        ]
    );

    Ok(())
}

#[allow(clippy::redundant_clone)]
#[test]
fn add_remove_lps() -> StdResult<()> {
    let mut deps = init_contract(None, None);
    let env = mock_env();

    let info = mock_info("addr0000", &[]);
    let owner = mock_info("owner", &[]);

    let err = execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        ExecuteMsg::UpdateConfig {
            factory: None,
            remove_factory: None,
            upsert_lps: None,
            delete_lps: None,
            insert_routes: None,
            delete_routes: None,
        },
    )
    .unwrap_err();
    assert_eq!(err, StdError::generic_err("unauthorized: sender is not owner").into());

    execute(
        deps.as_mut(),
        env.clone(),
        owner.clone(),
        ExecuteMsg::UpdateConfig {
            factory: None,
            remove_factory: None,
            upsert_lps: Some(vec![LpInit {
                slippage_tolerance: Decimal::percent(1),
                pair_contract: "pair_astro_token".to_string(),
                commission_bps: 50,
                wanted_token: astro(),
            }]),
            delete_lps: None,
            insert_routes: None,
            delete_routes: None,
        },
    )
    .expect("should update lps");

    let msg = QueryMsg::GetLps {
        start_after: None,
        limit: None,
    };
    let lp_configs: Vec<LpConfig> = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;

    assert_eq!(
        lp_configs,
        vec![
            LpConfig {
                pair_info: PairInfo {
                    asset_infos: vec![astro(), token()],
                    contract_addr: Addr::unchecked("pair_astro_token"),
                    liquidity_token: Addr::unchecked("astro_token_lp"),
                    pair_type: astroport::factory::PairType::Xyk {}
                },
                commission_bps: 50,
                slippage_tolerance: Decimal::percent(1),
                wanted_token: astro()
            },
            LpConfig {
                pair_info: PairInfo {
                    asset_infos: vec![token(), uluna()],
                    contract_addr: Addr::unchecked("pair_contract"),
                    liquidity_token: Addr::unchecked("liquidity_token"),
                    pair_type: astroport::factory::PairType::Xyk {}
                },
                commission_bps: 30,
                slippage_tolerance: Decimal::percent(1),
                wanted_token: uluna()
            }
        ]
    );

    execute(
        deps.as_mut(),
        env.clone(),
        owner.clone(),
        ExecuteMsg::UpdateConfig {
            factory: None,
            remove_factory: None,
            upsert_lps: None,
            delete_lps: Some(vec!["not_existing".to_string()]),
            insert_routes: None,
            delete_routes: None,
        },
    )
    .expect_err("should not update lps");

    execute(
        deps.as_mut(),
        env.clone(),
        owner.clone(),
        ExecuteMsg::UpdateConfig {
            factory: None,
            remove_factory: None,
            upsert_lps: None,
            delete_lps: Some(vec!["astro_token_lp".to_string()]),
            insert_routes: None,
            delete_routes: None,
        },
    )
    .expect("should update lps");

    let msg = QueryMsg::GetLps {
        start_after: None,
        limit: None,
    };
    let lp_configs: Vec<LpConfig> = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;

    assert_eq!(
        lp_configs,
        vec![LpConfig {
            pair_info: PairInfo {
                asset_infos: vec![token(), uluna()],
                contract_addr: Addr::unchecked("pair_contract"),
                liquidity_token: Addr::unchecked("liquidity_token"),
                pair_type: astroport::factory::PairType::Xyk {}
            },
            commission_bps: 30,
            slippage_tolerance: Decimal::percent(1),
            wanted_token: uluna()
        }]
    );

    Ok(())
}

#[allow(clippy::redundant_clone)]
#[test]
fn add_remove_routes() -> StdResult<()> {
    let mut deps = init_contract(None, None);
    let env = mock_env();

    let owner = mock_info("owner", &[]);

    execute(
        deps.as_mut(),
        env.clone(),
        owner.clone(),
        ExecuteMsg::UpdateConfig {
            factory: None,
            remove_factory: None,
            upsert_lps: None,
            delete_lps: None,
            insert_routes: None,
            delete_routes: Some(vec![
                RouteDelete {
                    from: any(),
                    to: uluna(),
                    both: Some(true),
                },
                RouteDelete {
                    from: astro(),
                    to: token(),
                    both: Some(true),
                },
                RouteDelete {
                    from: astro(),
                    to: uluna(),
                    both: Some(false),
                },
            ]),
        },
    )
    .expect("should update lps");

    let msg = QueryMsg::GetRoutes {
        start_after: None,
        limit: None,
    };
    let routes: Vec<RouteResponseItem> = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;

    assert_eq!(
        routes,
        vec![
            RouteResponseItem {
                key: ("uluna".to_string(), "astro".to_string()),
                route_type: RouteTypeResponseItem::Path {
                    router: "router".to_string(),
                    router_type: RouterType::AstroSwap,
                    route: vec!["uluna".to_string(), "any".to_string(), "astro".to_string()]
                }
            },
            RouteResponseItem {
                key: ("ibc/token".to_string(), "uluna".to_string()),
                route_type: RouteTypeResponseItem::PairProxy {
                    pair_contract: "pair0002".to_string(),
                    asset_infos: vec!["uluna".to_string(), "ibc/token".to_string()]
                }
            }
        ]
    );

    let err = execute(
        deps.as_mut(),
        env.clone(),
        owner.clone(),
        ExecuteMsg::UpdateConfig {
            factory: None,
            remove_factory: None,
            upsert_lps: None,
            delete_lps: None,
            insert_routes: Some(vec![RouteInit::Path {
                router: "router".to_string(),
                router_type: RouterType::AstroSwap,
                route: vec![astro(), any(), token(), uluna()],
            }]),
            delete_routes: None,
        },
    )
    .expect_err("shouldnt add routes");

    assert_eq!(err, StdError::generic_err("Route already registered").into());

    execute(
        deps.as_mut(),
        env.clone(),
        owner.clone(),
        ExecuteMsg::UpdateConfig {
            factory: None,
            remove_factory: None,
            upsert_lps: None,
            delete_lps: None,
            delete_routes: Some(vec![RouteDelete {
                from: astro(),
                to: uluna(),
                both: Some(true),
            }]),
            insert_routes: Some(vec![RouteInit::Path {
                router: "router".to_string(),
                router_type: RouterType::AstroSwap,
                route: vec![astro(), any(), token(), uluna()],
            }]),
        },
    )
    .unwrap();

    let msg = QueryMsg::GetRoutes {
        start_after: None,
        limit: None,
    };
    let routes: Vec<RouteResponseItem> = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;

    assert_eq!(
        routes,
        vec![
            RouteResponseItem {
                key: ("astro".to_string(), "uluna".to_string()),
                route_type: RouteTypeResponseItem::Path {
                    router: "router".to_string(),
                    router_type: RouterType::AstroSwap,
                    route: vec![
                        "astro".to_string(),
                        "any".to_string(),
                        "token".to_string(),
                        "uluna".to_string()
                    ]
                }
            },
            RouteResponseItem {
                key: ("uluna".to_string(), "astro".to_string()),
                route_type: RouteTypeResponseItem::Path {
                    router: "router".to_string(),
                    router_type: RouterType::AstroSwap,
                    route: vec![
                        "uluna".to_string(),
                        "token".to_string(),
                        "any".to_string(),
                        "astro".to_string()
                    ]
                }
            },
            RouteResponseItem {
                key: ("ibc/token".to_string(), "uluna".to_string()),
                route_type: RouteTypeResponseItem::PairProxy {
                    pair_contract: "pair0002".to_string(),
                    asset_infos: vec!["uluna".to_string(), "ibc/token".to_string()]
                }
            }
        ]
    );

    Ok(())
}

#[allow(clippy::redundant_clone)]
#[test]
fn compound() -> Result<(), ContractError> {
    let mut deps = init_contract(None, None);

    deps.querier.with_balance(&[(&String::from(MOCK_CONTRACT_ADDR), &[coin(1000000, "uluna")])]);

    let msg = ExecuteMsg::Compound {
        lp_token: "liquidity_token".to_string(),
        rewards: vec![uluna_amount(1000000)],
        receiver: None,
        no_swap: None,
        slippage_tolerance: None,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[coin(1000000u128, "uluna")]);

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::OptimalSwap {
                    lp_token: "liquidity_token".to_string()
                }))?,
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::ProvideLiquidity {
                    prev_balances: vec![token_amount(0), uluna_amount(0)],
                    receiver: "addr0000".to_string(),
                    slippage_tolerance: None,
                    lp_token: "liquidity_token".to_string()
                }))?,
            }),
        ]
    );

    deps.querier.with_balance(&[(
        &String::from(MOCK_CONTRACT_ADDR),
        &[
            Coin {
                denom: "uluna".to_string(),
                amount: Uint128::new(1000008),
            },
            Coin {
                denom: "ibc/token".to_string(),
                amount: Uint128::new(1000),
            },
        ],
    )]);

    deps.querier.with_token_balances(&[(
        &String::from("token"),
        &[(&String::from(MOCK_CONTRACT_ADDR), &Uint128::new(9))],
    )]);

    let msg = ExecuteMsg::Compound {
        rewards: vec![Asset {
            info: AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            amount: Uint128::from(1000000u128),
        }],
        receiver: None,
        no_swap: Some(true),
        slippage_tolerance: Some(Decimal::percent(2)),
        lp_token: "liquidity_token".to_string(),
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            funds: vec![],
            msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::ProvideLiquidity {
                prev_balances: vec![
                    token_asset(Addr::unchecked("token"), Uint128::from(9u128)),
                    native_asset("uluna".to_string(), Uint128::from(8u128)),
                ],
                receiver: "addr0000".to_string(),
                slippage_tolerance: Some(Decimal::percent(2)),
                lp_token: "liquidity_token".to_string()
            }))?,
        }),]
    );

    Ok(())
}

#[allow(clippy::redundant_clone)]
#[test]
fn compound_native_proxy() -> Result<(), ContractError> {
    let mut deps = init_contract(None, None);
    let env = mock_env();

    let info = mock_info(
        "addr0000",
        &[Coin {
            denom: "uluna".to_string(),
            amount: Uint128::from(1000000u128),
        }],
    );

    deps.querier.with_balance(&[(
        &String::from(MOCK_CONTRACT_ADDR),
        &[coin(1000008, "uluna"), coin(1000, "ibc/token")],
    )]);

    deps.querier.with_token_balances(&[(
        &String::from("token"),
        &[(&String::from(MOCK_CONTRACT_ADDR), &Uint128::new(9))],
    )]);

    let msg = ExecuteMsg::Compound {
        rewards: vec![uluna_amount(1000000), ibc_amount(1000)],
        receiver: None,
        no_swap: Some(true),
        slippage_tolerance: Some(Decimal::percent(2)),
        lp_token: "liquidity_token".to_string(),
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap_err();

    assert_eq!(
        res.to_string(),
        "Generic error: Native token balance mismatch between the argument and the transferred"
            .to_string()
    );

    let info = mock_info("addr0000", &[coin(1000000, "uluna"), coin(1000, "ibc/token")]);

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg)?;

    let pair = Pair(Addr::unchecked("pair0002"));

    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        vec![
            pair.swap_msg(&ibc_amount(1000), None, Some(Decimal::percent(50)), None)?,
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::ProvideLiquidity {
                    prev_balances: vec![token_amount(9), uluna_amount(8)],
                    receiver: "addr0000".to_string(),
                    slippage_tolerance: Some(Decimal::percent(2)),
                    lp_token: "liquidity_token".to_string()
                }))?,
            }),
        ]
    );

    Ok(())
}

#[allow(clippy::redundant_clone)]
#[test]
fn compound_token_path() -> Result<(), ContractError> {
    let mut deps = init_contract(None, None);
    let env = mock_env();
    let state = State::default();

    deps.querier.with_balance(&[(
        &String::from(MOCK_CONTRACT_ADDR),
        &[coin(1000008, "uluna"), coin(1000, "ibc/token")],
    )]);

    deps.querier.with_token_balances(&[(
        &String::from("astro"),
        &[(&String::from(MOCK_CONTRACT_ADDR), &Uint128::new(109))],
    )]);

    let msg = ExecuteMsg::Compound {
        rewards: vec![uluna_amount(1000000), ibc_amount(1000), astro_amount(109)],
        receiver: None,
        no_swap: Some(true),
        slippage_tolerance: Some(Decimal::percent(2)),
        lp_token: "liquidity_token".to_string(),
    };

    let info = mock_info("addr0000", &[coin(1000000, "uluna"), coin(1000, "ibc/token")]);

    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg)?;

    let config = state.routes.load(&deps.storage, (astro().as_bytes(), uluna().as_bytes()))?;

    assert_eq!(
        config,
        RouteConfig {
            key: (astro(), uluna()),
            route_type: RouteType::Path {
                router: Router(Addr::unchecked("router")),
                router_type: RouterType::AstroSwap,
                route: vec![astro(), any(), uluna()]
            }
        }
    );

    let pair = Pair(Addr::unchecked("pair0002"));

    let astro = astro_amount(109);
    let transfer = astro.transfer_from_msg(&info.sender, &env.contract.address)?;

    assert_eq!(res.messages.len(), 4);

    assert_eq!(
        res.messages[0].msg,
        pair.swap_msg(
            &native_asset("ibc/token".to_string(), Uint128::new(1000)),
            None,
            Some(Decimal::percent(50)),
            None
        )?
    );

    assert_eq!(res.messages[1].msg, transfer);
    assert_eq!(
        res.messages[2].msg,
        config.create_swap(&astro_amount(109), Decimal::percent(50), None)?
    );

    match res.messages[3].msg.clone() {
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            funds,
            msg,
        }) => {
            assert_eq!(contract_addr, env.contract.address.to_string());
            assert_eq!(funds.len(), 0);

            let sub_msg: ExecuteMsg = from_binary(&msg).unwrap();

            assert_eq!(
                sub_msg,
                ExecuteMsg::Callback(CallbackMsg::ProvideLiquidity {
                    prev_balances: vec![
                        token_asset(Addr::unchecked("token"), Uint128::from(0u128)),
                        native_asset("uluna".to_string(), Uint128::from(8u128)),
                    ],
                    receiver: "addr0000".to_string(),
                    slippage_tolerance: Some(Decimal::percent(2)),
                    lp_token: "liquidity_token".to_string()
                })
            );
        },

        _ => panic!("DO NOT ENTER HERE"),
    }
    Ok(())
}

#[allow(clippy::redundant_clone)]
#[test]
fn compound_failed() -> Result<(), ContractError> {
    let mut deps = init_contract(None, None);
    let env = mock_env();

    deps.querier.with_balance(&[(
        &String::from(MOCK_CONTRACT_ADDR),
        &[coin(1000008, "uluna"), coin(1000, "ibc/token")],
    )]);

    deps.querier.with_token_balances(&[(
        &String::from("astro"),
        &[(&String::from(MOCK_CONTRACT_ADDR), &Uint128::new(109))],
    )]);

    let msg = ExecuteMsg::Compound {
        rewards: vec![uluna_amount(1000000), ibc_amount(1000), astro_amount(109)],
        receiver: None,
        no_swap: Some(true),
        slippage_tolerance: Some(Decimal::percent(2)),
        lp_token: "unknown".to_string(),
    };

    let info = mock_info("addr0000", &[coin(1000000, "uluna"), coin(1000, "ibc/token")]);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();

    assert_eq!(res, StdError::generic_err("did not find lp unknown").into());

    let msg = ExecuteMsg::Compound {
        rewards: vec![unknown_amount(123)],
        receiver: None,
        no_swap: Some(true),
        slippage_tolerance: Some(Decimal::percent(2)),
        lp_token: "liquidity_token".to_string(),
    };

    let info = mock_info("addr0000", &[coin(1000000, "uluna"), coin(1000, "ibc/token")]);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();

    assert_eq!(res, StdError::generic_err("Querier contract error: unknown pair").into());

    Ok(())
}

fn astro() -> AssetInfo {
    token_asset_info(Addr::unchecked("astro"))
}
fn astro_amount(amount: u128) -> Asset {
    token_asset(Addr::unchecked("astro"), Uint128::new(amount))
}
fn token() -> AssetInfo {
    token_asset_info(Addr::unchecked("token"))
}
fn token_amount(amount: u128) -> Asset {
    token_asset(Addr::unchecked("token"), Uint128::new(amount))
}
fn any() -> AssetInfo {
    token_asset_info(Addr::unchecked("any"))
}
fn _any_amount(amount: u128) -> Asset {
    token_asset(Addr::unchecked("any"), Uint128::new(amount))
}

fn uluna() -> AssetInfo {
    native_asset_info("uluna".to_string())
}
fn uluna_amount(amount: u128) -> Asset {
    native_asset("uluna".to_string(), Uint128::new(amount))
}
fn ibc() -> AssetInfo {
    native_asset_info("ibc/token".to_string())
}
fn ibc_amount(amount: u128) -> Asset {
    native_asset("ibc/token".to_string(), Uint128::new(amount))
}

fn _unknown() -> AssetInfo {
    token_asset_info(Addr::unchecked("unknown"))
}
fn unknown_amount(amount: u128) -> Asset {
    token_asset(Addr::unchecked("unknown"), Uint128::new(amount))
}

#[test]
fn optimal_swap() -> Result<(), ContractError> {
    let mut deps = init_contract(None, None);
    let env = mock_env();

    deps.querier.with_balance(&[(&String::from("pair_contract"), &[coin(1000000000, "uluna")])]);

    deps.querier.with_token_balances(&[(
        &String::from("token"),
        &[
            (&String::from(MOCK_CONTRACT_ADDR), &Uint128::new(1000000)),
            (&String::from("pair_contract"), &Uint128::new(1000000000)),
        ],
    )]);

    let msg = ExecuteMsg::Callback(CallbackMsg::OptimalSwap {
        lp_token: "liquidity_token".to_string(),
    });

    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert_eq!(res, Err(ContractError::Unauthorized {}));

    let info = mock_info(env.contract.address.as_str(), &[]);
    let res = execute(deps.as_mut(), env, info, msg)?;

    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "token".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: "pair_contract".to_string(),
                amount: Uint128::new(500626),
                msg: to_binary(&AstroportPairCw20HookMsg::Swap {
                    ask_asset_info: None,
                    belief_price: None,
                    max_spread: Some(Decimal::percent(50)),
                    to: None,
                })?
            })?,
        }),]
    );

    Ok(())
}

#[test]
fn provide_liquidity() -> Result<(), ContractError> {
    let mut deps = init_contract(Some("pair_contract_2".to_string()), None);
    let env = mock_env();

    deps.querier.with_balance(&[
        (
            &String::from("pair_contract_2"),
            &[coin(1000000000, "uluna"), coin(1000000000, "ibc/token")],
        ),
        (&String::from(MOCK_CONTRACT_ADDR), &[coin(1000001, "uluna"), coin(2000002, "ibc/token")]),
    ]);

    let info = mock_info("addr0000", &[]);

    let msg = ExecuteMsg::Callback(CallbackMsg::ProvideLiquidity {
        lp_token: "liquidity_token_3".to_string(),
        receiver: "sender".to_string(),
        prev_balances: vec![ibc_amount(2), uluna_amount(1)],
        slippage_tolerance: None,
    });

    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert_eq!(res, Err(ContractError::Unauthorized {}));

    let info = mock_info(env.contract.address.as_str(), &[]);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "pair_contract_2".to_string(),
            funds: vec![coin(2000000, "ibc/token"), coin(1000000, "uluna"),],
            msg: to_binary(&AstroportPairExecuteMsg::ProvideLiquidity {
                assets: vec![uluna_amount(1000000u128), ibc_amount(2000000u128)],
                slippage_tolerance: Some(Decimal::percent(1)),
                auto_stake: None,
                receiver: Some("sender".to_string()),
            })?,
        }),]
    );

    deps.querier.with_balance(&[
        (
            &String::from("pair_contract_2"),
            &[coin(1000000000, "uluna"), coin(1000000000, "ibc/token")],
        ),
        (&String::from(MOCK_CONTRACT_ADDR), &[coin(1000001, "uluna"), coin(2, "ibc/token")]),
    ]);

    let res = execute(deps.as_mut(), env, info, msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "pair_contract_2".to_string(),
            funds: vec![coin(1000000, "uluna"),],
            msg: to_binary(&AstroportPairExecuteMsg::ProvideLiquidity {
                assets: vec![uluna_amount(1000000u128), ibc_amount(0u128)],
                slippage_tolerance: Some(Decimal::percent(1)),
                auto_stake: None,
                receiver: Some("sender".to_string()),
            })?,
        }),]
    );

    Ok(())
}

#[test]
fn test_get_swap_amount() -> StdResult<()> {
    let amount_a = Uint256::from(1146135045u128);
    let amount_b = Uint256::from(9093887u128);
    let pool_a = Uint256::from(114613504500u128);
    let pool_b = Uint256::from(909388700u128);
    let commission_bps = 30u64;

    let result = get_swap_amount(amount_a, amount_b, pool_a, pool_b, commission_bps)?;

    assert_eq!(result, Uint128::zero());

    Ok(())
}

#[allow(clippy::redundant_clone)]
#[test]
fn test_compound_simulation_proxy() -> StdResult<()> {
    let mut deps = init_contract(None, Some(token()));
    let env = mock_env();

    deps.querier.with_balance(&[(&String::from("pair_contract"), &[coin(1000000000, "uluna")])]);
    deps.querier.with_token_balances(&[
        (&String::from("token"), &[(&String::from("pair_contract"), &Uint128::new(1000000000))]),
        (&String::from("liquidity_token"), &[(&String::from("xxxx"), &Uint128::new(1000000000))]),
    ]);

    let msg = QueryMsg::CompoundSimulation {
        lp_token: "liquidity_token".to_string(),
        rewards: vec![astro_amount(100)],
    };

    let res: CompoundSimulationResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;

    assert_eq!(
        res,
        CompoundSimulationResponse {
            lp_amount: Uint128::new(499122),
            swap_asset_a_amount: Uint128::new(500626),
            swap_asset_b_amount: Uint128::new(0),
            return_a_amount: Uint128::new(0),
            return_b_amount: Uint128::new(498874)
        }
    );

    Ok(())
}

#[allow(clippy::redundant_clone)]
#[test]
fn test_compound_simulation_path() -> StdResult<()> {
    let mut deps = init_contract(None, None);
    let env = mock_env();

    deps.querier.with_balance(&[(&String::from("pair_contract"), &[coin(1000000000, "uluna")])]);
    deps.querier.with_token_balances(&[
        (&String::from("token"), &[(&String::from("pair_contract"), &Uint128::new(1000000000))]),
        (&String::from("liquidity_token"), &[(&String::from("xxxx"), &Uint128::new(1000000000))]),
    ]);

    let msg = QueryMsg::CompoundSimulation {
        lp_token: "liquidity_token".to_string(),
        rewards: vec![astro_amount(100)],
    };

    let res: CompoundSimulationResponse = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;

    assert_eq!(
        res,
        CompoundSimulationResponse {
            lp_amount: Uint128::new(499122),
            swap_asset_a_amount: Uint128::new(0),
            swap_asset_b_amount: Uint128::new(500626),
            return_a_amount: Uint128::new(498874),
            return_b_amount: Uint128::new(0)
        }
    );

    Ok(())
}
