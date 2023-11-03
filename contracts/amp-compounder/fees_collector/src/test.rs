use astroport::asset::{native_asset_info, token_asset_info, AssetInfo, AssetInfoExt};
use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, to_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, OwnedDeps, Response, StdError,
    Timestamp, Uint128, WasmMsg,
};
use eris::adapters::asset::AssetEx;
use eris::adapters::compounder::Compounder;
use eris::fees_collector::{AssetWithLimit, ExecuteMsg, InstantiateMsg, QueryMsg, TargetConfig};
use eris::helper::funds_or_allowance;
use eris::hub::ExecuteMsg as HubExecuteMsg;

use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::mock_querier::{mock_dependencies, WasmMockQuerier};
use crate::state::{Config, CONFIG};

type TargetConfigUnchecked = TargetConfig;
type TargetConfigChecked = TargetConfig;

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
const ZAPPER_1: &str = "zapper_1";
const ZAPPER_2: &str = "zapper_2";

#[test]
fn test() -> Result<(), ContractError> {
    let mut deps = mock_dependencies();
    create(&mut deps)?;
    config(&mut deps)?;
    owner(&mut deps)?;
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
                asset_override: None,
            },
            TargetConfigUnchecked::new(USER_2.to_string(), 2),
            TargetConfigUnchecked::new(USER_3.to_string(), 3),
        ]),
        zapper: None,
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
                asset_override: None,
            },
            TargetConfigUnchecked::new(USER_2.to_string(), 2),
            TargetConfigUnchecked::new(USER_3.to_string(), 3),
        ]),
        zapper: None,
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
                use_compound_proxy: None,
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

#[test]
fn test_distribute_first() -> Result<(), ContractError> {
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
                asset_override: None,
            },
            TargetConfigUnchecked::new_asset(
                USER_2.to_string(),
                2,
                native_asset_info(IBC_TOKEN.to_string()),
            ),
            TargetConfigUnchecked::new_asset(
                USER_3.to_string(),
                3,
                native_asset_info(IBC_TOKEN.to_string()),
            ),
        ]),
        zapper: None,
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
                asset_override: None,
            },
            TargetConfigUnchecked::new_asset(
                USER_2.to_string(),
                2,
                native_asset_info(IBC_TOKEN.to_string()),
            ),
            TargetConfigUnchecked::new_asset(
                USER_3.to_string(),
                3,
                native_asset_info(IBC_TOKEN.to_string()),
            ),
        ]),
        zapper: None,
        max_spread: None,
    };

    execute(deps.as_mut(), mock_env(), mock_info(USER_1, &[]), msg).unwrap();

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

    // distribute fee only
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(OPERATOR_1, &[]),
        ExecuteMsg::Collect {
            assets: vec![AssetWithLimit {
                info: native_asset_info(IBC_TOKEN.to_string()),
                limit: None,
                use_compound_proxy: None,
            }],
        },
    )?;

    // no funds yet
    assert_eq!(res.messages.len(), 3);
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        vec![
            CosmosMsg::Bank(BankMsg::Send {
                to_address: USER_2.to_string(),
                amount: vec![Coin {
                    denom: IBC_TOKEN.to_string(),
                    amount: Uint128::from(40_000000u128),
                }]
            }),
            CosmosMsg::Bank(BankMsg::Send {
                to_address: USER_3.to_string(),
                amount: vec![Coin {
                    denom: IBC_TOKEN.to_string(),
                    amount: Uint128::from(60_000000u128),
                }]
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::DistributeFees {})?,
            }),
        ]
    );

    // set balance
    deps.querier.set_balance(
        IBC_TOKEN.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        Uint128::from(0_000000u128),
    );

    // distribute fees without reaching min
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::DistributeFees {},
    )?;
    assert_eq!(res.messages.len(), 0);

    Ok(())
}

#[test]
fn test_distribute_first_token() -> Result<(), ContractError> {
    let mut deps = mock_dependencies();
    create(&mut deps)?;

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
                asset_override: Some(token1()),
            },
            TargetConfigUnchecked::new(USER_2.to_string(), 2),
            TargetConfigUnchecked::new_asset(USER_3.to_string(), 3, token1()),
        ]),
        zapper: None,
        max_spread: None,
    };

    execute(deps.as_mut(), mock_env(), mock_info(USER_1, &[]), msg).unwrap();

    // set balance
    deps.querier.set_balance(
        TOKEN_1.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        Uint128::from(100_000000u128),
    );

    deps.querier.set_balance(
        TOKEN_1.to_string(),
        "filler".to_string(),
        Uint128::from(8_500000u128),
    );

    // distribute fee only
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(OPERATOR_1, &[]),
        ExecuteMsg::Collect {
            assets: vec![AssetWithLimit {
                info: token1(),
                limit: None,
                use_compound_proxy: None,
            }],
        },
    )?;

    let transfer = token1()
        .with_balance(1_500000u128)
        .transfer_msg_target(&Addr::unchecked("filler".to_string()), None)
        .unwrap();
    let transfer2 = token1()
        .with_balance(98_500000u128)
        .transfer_msg_target(&Addr::unchecked(USER_3.to_string()), None)
        .unwrap();

    // no funds yet
    assert_eq!(res.messages.len(), 3);
    assert_eq!(
        res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(),
        vec![
            transfer,
            transfer2,
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::DistributeFees {})?,
            }),
        ]
    );

    // set balance
    deps.querier.set_balance(
        TOKEN_1.to_string(),
        MOCK_CONTRACT_ADDR.to_string(),
        Uint128::from(0_000000u128),
    );

    // distribute fees without reaching min
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(MOCK_CONTRACT_ADDR, &[]),
        ExecuteMsg::DistributeFees {},
    )?;
    assert_eq!(res.messages.len(), 0);

    Ok(())
}

fn token1() -> AssetInfo {
    token_asset_info(Addr::unchecked(TOKEN_1))
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
        zapper: ZAPPER_1.to_string(),
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
                TargetConfigChecked::new(USER_2, 2),
                TargetConfigChecked::new(USER_3, 3)
            ],
            stablecoin: AssetInfo::NativeToken {
                denom: IBC_TOKEN.to_string(),
            },
            max_spread: Decimal::percent(1),
            compound_proxy: Addr::unchecked(ZAPPER_1),
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
        zapper: None,
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
        zapper: None,
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert!(res.is_ok());

    let msg = ExecuteMsg::UpdateConfig {
        operator: None,
        factory_contract: None,
        target_list: Some(vec![TargetConfigUnchecked::new(USER_1.to_string(), 1)]),
        max_spread: None,
        zapper: None,
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone());
    assert!(res.is_ok());

    let msg = ExecuteMsg::UpdateConfig {
        operator: None,
        factory_contract: None,
        target_list: None,
        max_spread: Some(Decimal::percent(5)),
        zapper: None,
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
            max_spread: Decimal::percent(5),
            compound_proxy: Addr::unchecked(ZAPPER_1),
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
        zapper: Some(ZAPPER_2.to_string()),
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
            max_spread: Decimal::percent(1),
            compound_proxy: Addr::unchecked(ZAPPER_2),
        }
    );

    Ok(())
}

fn stablecoin() -> AssetInfo {
    AssetInfo::NativeToken {
        denom: IBC_TOKEN.to_string(),
    }
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

fn collect(
    deps: &mut OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
) -> Result<(), ContractError> {
    let env = mock_env();

    let msg = ExecuteMsg::Collect {
        assets: vec![AssetWithLimit {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked(TOKEN_1),
            },
            limit: None,
            use_compound_proxy: None,
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
    let assets = vec![token1().with_balance(1000000u128)];
    let mut swap_msgs = get_swap_msgs(assets)?;

    swap_msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        funds: vec![],
        msg: to_binary(&ExecuteMsg::DistributeFees {})?,
    }));

    assert_eq!(res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(), swap_msgs);

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
            use_compound_proxy: None,
        }],
    };

    // collect success
    let res = execute(deps.as_mut(), env.clone(), info, msg)?;

    let assets = vec![token_asset_info(Addr::unchecked(TOKEN_2)).with_balance(1500000u128)];
    let mut swap_msgs = get_swap_msgs(assets)?;
    swap_msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        funds: vec![],
        msg: to_binary(&ExecuteMsg::DistributeFees {})?,
    }));

    assert_eq!(res.messages.into_iter().map(|it| it.msg).collect::<Vec<CosmosMsg>>(), swap_msgs);

    Ok(())
}

fn get_swap_msgs(assets: Vec<astroport::asset::Asset>) -> Result<Vec<CosmosMsg>, ContractError> {
    let (funds, mut allowances) =
        funds_or_allowance(&mock_env(), &Addr::unchecked(ZAPPER_2), &assets, None)?;
    let msg =
        Compounder(Addr::unchecked(ZAPPER_2)).multi_swap_msg(assets, stablecoin(), funds, None)?;
    allowances.push(msg);
    Ok(allowances)
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
        zapper: None,
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
