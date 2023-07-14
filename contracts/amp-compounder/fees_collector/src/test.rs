use astroport::asset::{native_asset_info, AssetInfo, PairInfo};
use astroport::factory::PairType;
use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, to_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, OwnedDeps, Response, StdError,
    Timestamp, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use eris::adapters::pair::CustomCw20HookMsg;
use eris::fees_collector::{AssetWithLimit, ExecuteMsg, InstantiateMsg, QueryMsg, TargetConfig};
use eris::hub::ExecuteMsg as HubExecuteMsg;

use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::mock_querier::{mock_dependencies, WasmMockQuerier};
use crate::state::{Config, CONFIG};

type TargetConfigUnchecked = TargetConfig<String>;
type TargetConfigChecked = TargetConfig<Addr>;

const OWNER: &str = "owner";
const OPERATOR_1: &str = "operator_1";
const OPERATOR_2: &str = "operator_2";
const USER_1: &str = "user_1";
const USER_2: &str = "user_2";
const USER_3: &str = "user_3";
const HUB_1: &str = "hub_1";
const FACTORY_1: &str = "factory_1";
const FACTORY_2: &str = "factory_2";
const TOKEN_1: &str = "token_1";
const TOKEN_2: &str = "token_2";
const IBC_TOKEN: &str = "ibc/stablecoin";

#[test]
fn test() -> Result<(), ContractError> {
    let mut deps = mock_dependencies();
    create(&mut deps)?;
    config(&mut deps)?;
    owner(&mut deps)?;
    bridges(&mut deps)?;
    collect(&mut deps)?;
    distribute_fees(&mut deps)?;
    distribute_fees_to_contract(&mut deps)?;

    Ok(())
}

#[test]
fn test_fillup() -> Result<(), ContractError> {
    let mut deps = mock_dependencies();
    create(&mut deps)?;

    let msg = ExecuteMsg::UpdateConfig {
        operator: None,
        factory_contract: None,
        target_list: Some(vec![
            TargetConfigUnchecked {
                addr: "filler".to_string(),
                weight: 1,
                msg: None,
                target_type: eris::fees_collector::TargetType::FillUpFirst {
                    filled_to: Uint128::new(10_000000),
                    min_fill: Some(Uint128::new(1_000000)),
                },
            },
            TargetConfigUnchecked::new(USER_2.to_string(), 2),
            TargetConfigUnchecked::new(USER_3.to_string(), 3),
        ]),
        max_spread: None,
    };

    let res = execute(deps.as_mut(), mock_env(), mock_info(USER_1, &[]), msg).unwrap_err();
    assert_eq!(res.to_string(), "Generic error: FillUp can't have a weight (1)");

    let msg = ExecuteMsg::UpdateConfig {
        operator: None,
        factory_contract: None,
        target_list: Some(vec![
            TargetConfigUnchecked {
                addr: "filler".to_string(),
                weight: 0,
                msg: None,
                target_type: eris::fees_collector::TargetType::FillUpFirst {
                    filled_to: Uint128::new(10_000000),
                    min_fill: Some(Uint128::new(1_000000)),
                },
            },
            TargetConfigUnchecked::new(USER_2.to_string(), 2),
            TargetConfigUnchecked::new(USER_3.to_string(), 3),
        ]),
        max_spread: None,
    };

    execute(deps.as_mut(), mock_env(), mock_info(USER_1, &[]), msg).unwrap();

    // distribute fee only
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(OPERATOR_1, &[]),
        ExecuteMsg::Collect {
            assets: vec![AssetWithLimit {
                info: native_asset_info(IBC_TOKEN.to_string()),
                limit: None,
            }],
        },
    )?;
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: MOCK_CONTRACT_ADDR.to_string(),
            funds: vec![],
            msg: to_binary(&ExecuteMsg::DistributeFees {})?,
        }),]
    );

    // set balance
    deps.querier.set_balance(
        IBC_TOKEN.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        Uint128::from(100_000000u128),
    );
    deps.querier.set_balance(
        IBC_TOKEN.to_string(),
        "filler".to_string(),
        Uint128::from(9_500000u128),
    );
    // distribute fees without reaching min
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::DistributeFees {},
    )?;
    assert_eq!(res.messages.len(), 2);
    assert_eq!(
        res.messages[0].msg,
        CosmosMsg::Bank(BankMsg::Send {
            to_address: USER_2.to_string(),
            amount: vec![Coin {
                denom: IBC_TOKEN.to_string(),
                amount: Uint128::from(40_000000u128),
            }]
        }),
    );
    assert_eq!(
        res.messages[1].msg,
        CosmosMsg::Bank(BankMsg::Send {
            to_address: USER_3.to_string(),
            amount: vec![Coin {
                denom: IBC_TOKEN.to_string(),
                amount: Uint128::from(60_000000u128),
            }]
        }),
    );

    // set balance
    deps.querier.set_balance(
        IBC_TOKEN.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        Uint128::from(100_000000u128),
    );
    deps.querier.set_balance(
        IBC_TOKEN.to_string(),
        "filler".to_string(),
        Uint128::from(2_400000u128),
    );
    // distribute fees without reaching min
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::DistributeFees {},
    )?;
    assert_eq!(res.messages.len(), 3);
    assert_eq!(
        res.messages[0].msg,
        CosmosMsg::Bank(BankMsg::Send {
            to_address: "filler".to_string(),
            amount: vec![Coin {
                denom: IBC_TOKEN.to_string(),
                amount: Uint128::from(7_600000u128),
            }]
        }),
    );
    assert_eq!(
        res.messages[1].msg,
        CosmosMsg::Bank(BankMsg::Send {
            to_address: USER_2.to_string(),
            amount: vec![Coin {
                denom: IBC_TOKEN.to_string(),
                amount: Uint128::from(36_960000u128),
            }]
        }),
    );
    assert_eq!(
        res.messages[2].msg,
        CosmosMsg::Bank(BankMsg::Send {
            to_address: USER_3.to_string(),
            amount: vec![Coin {
                denom: IBC_TOKEN.to_string(),
                amount: Uint128::from(55_440000u128),
            }]
        }),
    );

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
    let info = mock_info(USER_1, &[]);

    let instantiate_msg = InstantiateMsg {
        owner: USER_1.to_string(),
        factory_contract: FACTORY_1.to_string(),
        max_spread: Some(Decimal::percent(1)),
        operator: OPERATOR_1.to_string(),
        stablecoin: AssetInfo::NativeToken {
            denom: IBC_TOKEN.to_string(),
        },
        target_list: vec![
            TargetConfigUnchecked::new(USER_2.to_string(), 2),
            TargetConfigUnchecked::new(USER_3.to_string(), 3),
        ],
    };
    let res = instantiate(deps.as_mut(), env, info, instantiate_msg);
    assert!(res.is_ok());

    let config = CONFIG.load(deps.as_mut().storage)?;
    assert_eq!(
        config,
        Config {
            owner: Addr::unchecked(USER_1),
            operator: Addr::unchecked(OPERATOR_1),
            factory_contract: Addr::unchecked(FACTORY_1),
            target_list: vec![
                TargetConfigChecked::new(Addr::unchecked(USER_2), 2),
                TargetConfigChecked::new(Addr::unchecked(USER_3), 3)
            ],
            stablecoin: AssetInfo::NativeToken {
                denom: IBC_TOKEN.to_string(),
            },
            max_spread: Decimal::percent(1)
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
        operator: Some(OPERATOR_2.to_string()),
        factory_contract: None,
        target_list: None,
        max_spread: None,
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert_error(res, "Unauthorized");

    let info = mock_info(USER_1, &[]);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert!(res.is_ok());

    let msg = ExecuteMsg::UpdateConfig {
        operator: None,
        factory_contract: Some(FACTORY_2.to_string()),
        target_list: None,
        max_spread: None,
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert!(res.is_ok());

    let msg = ExecuteMsg::UpdateConfig {
        operator: None,
        factory_contract: None,
        target_list: Some(vec![TargetConfigUnchecked::new(USER_1.to_string(), 1)]),
        max_spread: None,
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert!(res.is_ok());

    let msg = ExecuteMsg::UpdateConfig {
        operator: None,
        factory_contract: None,
        target_list: None,
        max_spread: Some(Decimal::percent(5)),
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert!(res.is_ok());

    let msg = QueryMsg::Config {};
    let res: Config = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        Config {
            owner: Addr::unchecked(USER_1),
            operator: Addr::unchecked(OPERATOR_2),
            factory_contract: Addr::unchecked(FACTORY_2),
            target_list: vec![TargetConfigChecked::new(Addr::unchecked(USER_1), 1)],
            stablecoin: AssetInfo::NativeToken {
                denom: IBC_TOKEN.to_string(),
            },
            max_spread: Decimal::percent(5)
        }
    );

    let msg = ExecuteMsg::UpdateConfig {
        operator: Some(OPERATOR_1.to_string()),
        factory_contract: Some(FACTORY_1.to_string()),
        target_list: Some(vec![
            TargetConfigUnchecked::new(USER_2.to_string(), 2),
            TargetConfigUnchecked::new(USER_3.to_string(), 3),
        ]),
        max_spread: Some(Decimal::percent(1)),
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert!(res.is_ok());

    let msg = QueryMsg::Config {};
    let res: Config = from_binary(&query(deps.as_ref(), env.clone(), msg)?)?;
    assert_eq!(
        res,
        Config {
            owner: Addr::unchecked(USER_1),
            operator: Addr::unchecked(OPERATOR_1),
            factory_contract: Addr::unchecked(FACTORY_1),
            target_list: vec![
                TargetConfigChecked::new(Addr::unchecked(USER_2), 2),
                TargetConfigChecked::new(Addr::unchecked(USER_3), 3)
            ],
            stablecoin: AssetInfo::NativeToken {
                denom: IBC_TOKEN.to_string(),
            },
            max_spread: Decimal::percent(1)
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
    let config: Config = from_binary(&query(deps.as_ref(), env.clone(), QueryMsg::Config {})?)?;
    assert_eq!(OWNER, config.owner);
    Ok(())
}

#[allow(clippy::redundant_clone)]
fn bridges(
    deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
) -> Result<(), ContractError> {
    let env = mock_env();

    let msg = ExecuteMsg::UpdateBridges {
        add: Some(vec![(
            AssetInfo::Token {
                contract_addr: Addr::unchecked(TOKEN_1),
            },
            AssetInfo::Token {
                contract_addr: Addr::unchecked(TOKEN_2),
            },
        )]),
        remove: None,
    };

    // update bridges unauthorized
    let info = mock_info(USER_1, &[]);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert_error(res, "Unauthorized");

    deps.querier.set_pair(
        &[
            AssetInfo::Token {
                contract_addr: Addr::unchecked(TOKEN_1),
            },
            AssetInfo::Token {
                contract_addr: Addr::unchecked(TOKEN_2),
            },
        ],
        PairInfo {
            asset_infos: vec![
                AssetInfo::Token {
                    contract_addr: Addr::unchecked(TOKEN_1),
                },
                AssetInfo::Token {
                    contract_addr: Addr::unchecked(TOKEN_2),
                },
            ],
            contract_addr: Addr::unchecked("token1token2"),
            liquidity_token: Addr::unchecked("liquidity0000"),
            pair_type: PairType::Xyk {},
        },
    );

    deps.querier.set_pair(
        &[
            AssetInfo::Token {
                contract_addr: Addr::unchecked(TOKEN_2),
            },
            AssetInfo::NativeToken {
                denom: IBC_TOKEN.to_string(),
            },
        ],
        PairInfo {
            asset_infos: vec![
                AssetInfo::Token {
                    contract_addr: Addr::unchecked(TOKEN_2),
                },
                AssetInfo::NativeToken {
                    denom: IBC_TOKEN.to_string(),
                },
            ],
            contract_addr: Addr::unchecked("token2ibc"),
            liquidity_token: Addr::unchecked("liquidity0002"),
            pair_type: PairType::Stable {},
        },
    );

    let info = mock_info(OPERATOR_1, &[]);

    // update bridges
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert!(res.is_ok());

    // query bridges
    let bridges: Vec<(String, String)> =
        from_binary(&query(deps.as_ref(), env.clone(), QueryMsg::Bridges {})?)?;
    assert_eq!(vec![(TOKEN_1.to_string(), TOKEN_2.to_string())], bridges);

    let msg = ExecuteMsg::UpdateBridges {
        add: None,
        remove: Some(vec![AssetInfo::Token {
            contract_addr: Addr::unchecked(TOKEN_1),
        }]),
    };

    let res = execute(deps.as_mut(), env.clone(), info, msg);
    assert!(res.is_ok());

    // query bridges
    let bridges: Vec<(String, String)> =
        from_binary(&query(deps.as_ref(), env.clone(), QueryMsg::Bridges {})?)?;
    assert!(bridges.is_empty());

    Ok(())
}

fn collect(
    deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
) -> Result<(), ContractError> {
    let env = mock_env();

    // update bridges
    let info = mock_info(OPERATOR_1, &[]);
    let msg = ExecuteMsg::UpdateBridges {
        add: Some(vec![(
            AssetInfo::Token {
                contract_addr: Addr::unchecked(TOKEN_1),
            },
            AssetInfo::Token {
                contract_addr: Addr::unchecked(TOKEN_2),
            },
        )]),
        remove: None,
    };
    let res = execute(deps.as_mut(), env.clone(), info, msg);
    assert!(res.is_ok());

    let msg = ExecuteMsg::Collect {
        assets: vec![AssetWithLimit {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked(TOKEN_1),
            },
            limit: None,
        }],
    };

    let info = mock_info(USER_1, &[]);

    // unauthorized check
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert_error(res, "Unauthorized");

    // distribute fee only if no balance
    let info = mock_info(OPERATOR_1, &[]);
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            funds: vec![],
            msg: to_binary(&ExecuteMsg::DistributeFees {})?,
        }),]
    );

    // set balance
    deps.querier.set_balance(
        TOKEN_1.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        Uint128::from(1000000u128),
    );

    // collect success
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: TOKEN_1.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: "token1token2".to_string(),
                    amount: Uint128::new(1000000u128),
                    msg: to_binary(&CustomCw20HookMsg::Swap {
                        belief_price: None,
                        max_spread: Some(Decimal::percent(1)),
                        to: None,
                    })?
                })?,
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::SwapBridgeAssets {
                    assets: vec![AssetInfo::Token {
                        contract_addr: Addr::unchecked(TOKEN_2)
                    }],
                    depth: 0
                })?,
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::DistributeFees {})?,
            }),
        ]
    );

    // set balance
    deps.querier.set_balance(
        TOKEN_2.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        Uint128::from(2000000u128),
    );

    let msg = ExecuteMsg::Collect {
        assets: vec![AssetWithLimit {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked(TOKEN_2),
            },
            limit: Some(Uint128::from(1500000u128)),
        }],
    };

    // collect success
    let res = execute(deps.as_mut(), env.clone(), info, msg)?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: TOKEN_2.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: "token2ibc".to_string(),
                    amount: Uint128::new(1500000u128),
                    msg: to_binary(&CustomCw20HookMsg::Swap {
                        belief_price: None,
                        max_spread: Some(Decimal::percent(1)),
                        to: None,
                    })?
                })?,
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::DistributeFees {})?,
            }),
        ]
    );

    Ok(())
}

#[allow(clippy::redundant_clone)]
fn distribute_fees(
    deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
) -> Result<(), ContractError> {
    let env = mock_env();

    // set balance
    deps.querier.set_balance(
        IBC_TOKEN.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        Uint128::from(1000000u128),
    );

    let msg = ExecuteMsg::DistributeFees {};

    let info = mock_info(USER_1, &[]);

    // unauthorized check
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert_error(res, "Unauthorized");

    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone())?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [
            CosmosMsg::Bank(BankMsg::Send {
                to_address: USER_2.to_string(),
                amount: vec![Coin {
                    denom: IBC_TOKEN.to_string(),
                    amount: Uint128::from(400000u128),
                }]
            }),
            CosmosMsg::Bank(BankMsg::Send {
                to_address: USER_3.to_string(),
                amount: vec![Coin {
                    denom: IBC_TOKEN.to_string(),
                    amount: Uint128::from(600000u128),
                }]
            }),
        ]
    );

    Ok(())
}

#[allow(clippy::redundant_clone)]
fn distribute_fees_to_contract(
    deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
) -> Result<(), ContractError> {
    let env = mock_env();

    let owner = mock_info(OWNER, &[]);

    // set balance
    deps.querier.set_balance(
        IBC_TOKEN.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        Uint128::from(1000000u128),
    );

    let msg = ExecuteMsg::UpdateConfig {
        operator: None,
        factory_contract: None,
        target_list: Some(vec![
            TargetConfigUnchecked::new_msg(
                HUB_1.to_string(),
                1,
                Some(to_binary(&HubExecuteMsg::Donate {}).unwrap()),
            ),
            TargetConfigUnchecked::new(USER_1.to_string(), 4),
        ]),
        max_spread: None,
    };
    let res = execute(deps.as_mut(), env.clone(), owner.clone(), msg.clone());
    assert!(res.is_ok());

    let msg = ExecuteMsg::DistributeFees {};

    let info = mock_info(USER_1, &[]);
    // unauthorized check
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert_error(res, "Unauthorized");

    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone())?;
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        [
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HUB_1.to_string(),
                msg: to_binary(&HubExecuteMsg::Donate {}).unwrap(),
                funds: vec![Coin {
                    denom: IBC_TOKEN.to_string(),
                    amount: Uint128::from(200000u128),
                }],
            }),
            CosmosMsg::Bank(BankMsg::Send {
                to_address: USER_1.to_string(),
                amount: vec![Coin {
                    denom: IBC_TOKEN.to_string(),
                    amount: Uint128::from(800000u128),
                }]
            }),
        ]
    );

    Ok(())
}
