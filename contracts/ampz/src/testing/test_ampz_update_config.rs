use std::convert::TryInto;
use std::vec;

use astroport::asset::{native_asset_info, token_asset_info};
use cosmwasm_schema::schemars::Map;
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{coins, Addr};

use eris::ampz::{AstroportConfig, ConfigResponse, ExecuteMsg, FeeConfig, QueryMsg, TlaConfig};

use crate::constants::CONTRACT_DENOM;
use crate::contract::execute;

use crate::error::ContractError;
use crate::state::State;
use crate::testing::helpers::{mock_env_at_timestamp, query_helper, setup_test};

#[test]
fn check_default_config() {
    let deps = setup_test();
    // default config
    let res: ConfigResponse = query_helper(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(
        res,
        ConfigResponse {
            owner: "owner".to_string(),
            new_owner: None,
            hub: "hub".to_string(),
            farms: vec!["farm1".into(), "farm2".into()],
            controller: "controller".into(),
            zapper: "zapper".into(),
            astroport: AstroportConfig {
                generator: "generator".into(),
                coins: vec![
                    native_asset_info("uluna".into()),
                    token_asset_info(Addr::unchecked("astro")),
                ]
            },
            fee: FeeConfig {
                fee_bps: 100u16.try_into().unwrap(),
                operator_bps: 200u16.try_into().unwrap(),
                tla_source_fee: 750u16.try_into().unwrap(),
                receiver: "fee_receiver".into()
            },
            capapult: eris::ampz::CapapultConfig {
                market: "capapult_market".into(),
                overseer: "capapult_overseer".into(),
                custody: "capapult_custody".into(),
                stable_cw: "solid".into(),
            },
            arb_vault: "arb_vault".into(),
            creda: eris::ampz::CredaConfig {
                portfolio: Addr::unchecked("creda_portfolio"),
            },
            tla: TlaConfig {
                amp_luna_addr: Addr::unchecked("amp_luna"),
                compounder: Addr::unchecked("tla_compounder"),
                gauges: Map::new(),
            }
        }
    );
}

#[test]
fn check_update_config() {
    let mut deps = setup_test();
    // update all elements
    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("owner", &[]),
        ExecuteMsg::UpdateConfig {
            add_farms: Some(vec!["added".into()]),
            remove_farms: Some(vec!["farm1".into()]),
            controller: Some("new_controller".into()),
            zapper: Some("new_zapper".into()),

            astroport: Some(AstroportConfig {
                generator: "new_generator".into(),
                coins: vec![],
            }),
            fee: Some(FeeConfig {
                fee_bps: 10u16.try_into().unwrap(),
                operator_bps: 20u16.try_into().unwrap(),
                tla_source_fee: 750u16.try_into().unwrap(),
                receiver: "new_fee_receiver".into(),
            }),
            capapult: None,
            alliance: None,
            whitewhale: None,
            hub: Some("new_hub".into()),
            arb_vault: Some("new_arb_vault".into()),
            creda: None,
            tla: None,
        },
    )
    .unwrap_err();

    assert_eq!(res, ContractError::CannotAddAndRemoveFarms {});

    execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("owner", &[]),
        ExecuteMsg::UpdateConfig {
            add_farms: Some(vec!["added".into()]),
            remove_farms: None,
            controller: Some("new_controller".into()),
            zapper: Some("new_zapper".into()),

            astroport: Some(AstroportConfig {
                generator: "new_generator".into(),
                coins: vec![],
            }),
            fee: Some(FeeConfig {
                fee_bps: 10u16.try_into().unwrap(),
                operator_bps: 20u16.try_into().unwrap(),
                tla_source_fee: 750u16.try_into().unwrap(),
                receiver: "new_fee_receiver".into(),
            }),
            capapult: None,
            alliance: None,
            whitewhale: None,
            hub: Some("new_hub".into()),
            arb_vault: Some("new_arb_vault".into()),
            creda: None,
            tla: None,
        },
    )
    .unwrap();

    let res: ConfigResponse = query_helper(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(
        res,
        ConfigResponse {
            owner: "owner".to_string(),
            new_owner: None,
            hub: "new_hub".to_string(),
            farms: vec!["farm1".into(), "farm2".into(), "added".into()],
            controller: "new_controller".into(),
            zapper: "new_zapper".into(),
            astroport: AstroportConfig {
                generator: "new_generator".into(),
                coins: vec![]
            },
            fee: FeeConfig {
                fee_bps: 10u16.try_into().unwrap(),
                operator_bps: 20u16.try_into().unwrap(),
                tla_source_fee: 750u16.try_into().unwrap(),
                receiver: "new_fee_receiver".into()
            },
            capapult: eris::ampz::CapapultConfig {
                market: "capapult_market".into(),
                overseer: "capapult_overseer".into(),
                custody: "capapult_custody".into(),
                stable_cw: "solid".into(),
            },
            arb_vault: "new_arb_vault".into(),

            creda: eris::ampz::CredaConfig {
                portfolio: Addr::unchecked("creda_portfolio"),
            },
            tla: TlaConfig {
                amp_luna_addr: Addr::unchecked("amp_luna"),
                compounder: Addr::unchecked("tla_compounder"),
                gauges: Map::new(),
            }
        }
    );

    execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("owner", &[]),
        ExecuteMsg::UpdateConfig {
            add_farms: None,
            remove_farms: Some(vec!["farm1".into()]),
            controller: Some("new_controller".into()),
            zapper: Some("new_zapper".into()),

            astroport: Some(AstroportConfig {
                generator: "new_generator".into(),
                coins: vec![],
            }),
            fee: Some(FeeConfig {
                fee_bps: 10u16.try_into().unwrap(),
                operator_bps: 20u16.try_into().unwrap(),
                tla_source_fee: 750u16.try_into().unwrap(),
                receiver: "new_fee_receiver".into(),
            }),
            capapult: None,
            hub: Some("new_hub".into()),
            arb_vault: Some("new_arb_vault".into()),
            alliance: None,
            whitewhale: None,
            creda: None,
            tla: None,
        },
    )
    .unwrap();

    let res: ConfigResponse = query_helper(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(
        res,
        ConfigResponse {
            owner: "owner".to_string(),
            new_owner: None,
            hub: "new_hub".to_string(),
            farms: vec!["farm2".into(), "added".into()],
            controller: "new_controller".into(),
            zapper: "new_zapper".into(),
            astroport: AstroportConfig {
                generator: "new_generator".into(),
                coins: vec![]
            },
            fee: FeeConfig {
                fee_bps: 10u16.try_into().unwrap(),
                operator_bps: 20u16.try_into().unwrap(),
                tla_source_fee: 750u16.try_into().unwrap(),
                receiver: "new_fee_receiver".into()
            },
            capapult: eris::ampz::CapapultConfig {
                market: "capapult_market".into(),
                overseer: "capapult_overseer".into(),
                custody: "capapult_custody".into(),
                stable_cw: "solid".into(),
            },
            arb_vault: "new_arb_vault".into(),
            creda: eris::ampz::CredaConfig {
                portfolio: Addr::unchecked("creda_portfolio"),
            },
            tla: TlaConfig {
                amp_luna_addr: Addr::unchecked("amp_luna"),
                compounder: Addr::unchecked("tla_compounder"),
                gauges: Map::new(),
            }
        }
    );
}

#[test]
fn update_config_unauthorized() {
    let mut deps = setup_test();
    deps.querier.bank_querier.update_balance("user", coins(50, CONTRACT_DENOM));

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(1000),
        mock_info("user", &[]),
        ExecuteMsg::UpdateConfig {
            add_farms: None,
            remove_farms: None,
            controller: None,
            zapper: None,
            astroport: None,
            fee: None,
            hub: None,
            arb_vault: None,
            capapult: None,
            alliance: None,
            whitewhale: None,
            creda: None,
            tla: None,
        },
    )
    .unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});
}

#[test]
fn transferring_ownership() {
    let mut deps = setup_test();
    let state = State::default();

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake", &[]),
        ExecuteMsg::TransferOwnership {
            new_owner: "jake".to_string(),
        },
    )
    .unwrap_err();

    assert_eq!(err, ContractError::Unauthorized {});

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::TransferOwnership {
            new_owner: "jake".to_string(),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 0);

    let owner = state.owner.load(deps.as_ref().storage).unwrap();
    assert_eq!(owner, Addr::unchecked("owner"));
    let new_owner = state.new_owner.load(deps.as_ref().storage).unwrap();
    assert_eq!(new_owner, Addr::unchecked("jake"));

    // Check dropping ownership proposal
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("pumpkin", &[]),
        ExecuteMsg::DropOwnershipProposal {},
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::DropOwnershipProposal {},
    )
    .unwrap();
    assert_eq!(res.messages.len(), 0);

    let owner = state.owner.load(deps.as_ref().storage).unwrap();
    assert_eq!(owner, Addr::unchecked("owner"));
    let new_owner = state.new_owner.may_load(deps.as_ref().storage).unwrap();
    assert_eq!(new_owner, None);

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner", &[]),
        ExecuteMsg::TransferOwnership {
            new_owner: "jake".to_string(),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 0);

    let owner = state.owner.load(deps.as_ref().storage).unwrap();
    assert_eq!(owner, Addr::unchecked("owner"));
    let new_owner = state.new_owner.load(deps.as_ref().storage).unwrap();
    assert_eq!(new_owner, Addr::unchecked("jake"));

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("pumpkin", &[]),
        ExecuteMsg::AcceptOwnership {},
    )
    .unwrap_err();

    assert_eq!(err, ContractError::UnauthorizedSenderNotNewOwner {});

    let res =
        execute(deps.as_mut(), mock_env(), mock_info("jake", &[]), ExecuteMsg::AcceptOwnership {})
            .unwrap();

    assert_eq!(res.messages.len(), 0);

    let owner = state.owner.load(deps.as_ref().storage).unwrap();
    assert_eq!(owner, Addr::unchecked("jake"));
}
