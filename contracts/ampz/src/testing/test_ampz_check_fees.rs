use astroport::asset::native_asset;
use cosmwasm_std::testing::{mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{coins, Addr, Uint128};

use eris::adapters::hub::Hub;
use eris::ampz::ExecuteMsg;
use eris::constants::DAY;

use crate::contract::execute;
use eris::constants::CONTRACT_DENOM;

use crate::testing::helpers::{
    add_default_execution, finish_amplifier, mock_env_at_timestamp, setup_test,
};
use eris::adapters::asset::AssetEx;

#[test]
fn controller_executes_receives_no_fee() {
    let mut deps = setup_test();
    deps.querier.bank_querier.update_balance("user", coins(150, CONTRACT_DENOM));

    add_default_execution(&mut deps);

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("controller", &[]),
        ExecuteMsg::Execute {
            id: Uint128::new(1),
        },
    )
    .unwrap();

    // no claim as default execution is wallet
    // deposit + finish
    assert_eq!(res.messages.len(), 2);

    // need to execute finish callback to allow next execution
    deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, coins(100, CONTRACT_DENOM));
    let res = finish_amplifier(&mut deps, "controller");
    assert_eq!(res.messages.len(), 2);

    assert_eq!(
        res.messages[0].msg,
        // 1%+1% is sent to fee_receiver
        native_asset(CONTRACT_DENOM.into(), Uint128::new(3))
            .transfer_msg(&Addr::unchecked("fee_receiver"))
            .unwrap(),
    );

    assert_eq!(
        res.messages[1].msg,
        Hub(Addr::unchecked("hub")).bond_msg(CONTRACT_DENOM, 97, Some("user".into())).unwrap()
    );
}

#[test]
fn user_executes_no_controller_fee() {
    let mut deps = setup_test();
    deps.querier.bank_querier.update_balance("user", coins(150, CONTRACT_DENOM));

    add_default_execution(&mut deps);

    let res = execute(
        deps.as_mut(),
        // only user can execute before
        mock_env_at_timestamp(1000),
        mock_info("user", &[]),
        ExecuteMsg::Execute {
            id: Uint128::new(1),
        },
    )
    .unwrap();

    // claim + deposit + finish
    assert_eq!(res.messages.len(), 2);

    deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, coins(100, CONTRACT_DENOM));
    let res = finish_amplifier(&mut deps, "user");

    assert_eq!(res.messages.len(), 2);
    assert_eq!(
        res.messages[0].msg,
        // only 1% is sent to fee_receiver
        native_asset(CONTRACT_DENOM.into(), Uint128::new(1))
            .transfer_msg(&Addr::unchecked("fee_receiver"))
            .unwrap(),
    );

    assert_eq!(
        res.messages[1].msg,
        Hub(Addr::unchecked("hub")).bond_msg(CONTRACT_DENOM, 99, Some("user".into())).unwrap()
    );
}

#[test]
fn anyone_executes_multiple_fee() {
    let mut deps = setup_test();
    deps.querier.bank_querier.update_balance("user", coins(150, CONTRACT_DENOM));

    add_default_execution(&mut deps);

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(DAY),
        mock_info("anyone", &[]),
        ExecuteMsg::Execute {
            id: Uint128::new(1),
        },
    )
    .unwrap();

    // claim + deposit + finish
    assert_eq!(res.messages.len(), 2);

    deps.querier.bank_querier.update_balance(MOCK_CONTRACT_ADDR, coins(100, CONTRACT_DENOM));
    let res = finish_amplifier(&mut deps, "anyone");

    assert_eq!(res.messages.len(), 3);
    assert_eq!(
        res.messages[0].msg,
        // 1% is sent to fee_receiver
        native_asset(CONTRACT_DENOM.into(), Uint128::new(1))
            .transfer_msg(&Addr::unchecked("fee_receiver"))
            .unwrap(),
    );
    assert_eq!(
        res.messages[1].msg,
        // 1% is sent to executor
        native_asset(CONTRACT_DENOM.into(), Uint128::new(2))
            .transfer_msg(&Addr::unchecked("anyone"))
            .unwrap(),
    );

    assert_eq!(
        res.messages[2].msg,
        Hub(Addr::unchecked("hub")).bond_msg(CONTRACT_DENOM, 97, Some("user".into())).unwrap()
    );
}
