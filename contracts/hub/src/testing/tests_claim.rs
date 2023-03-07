use cosmwasm_std::attr;
use cosmwasm_std::testing::mock_info;

use eris::hub::{ClaimType, ExecuteMsg};

use crate::claim::ClaimExecuteMsg;
use crate::contract::execute;
use crate::error::ContractError;
use crate::testing::helpers::setup_test;

use super::helpers::mock_env_at_timestamp;

//--------------------------------------------------------------------------------------------------
// Execution
//--------------------------------------------------------------------------------------------------

#[test]
fn check_claim() {
    let mut deps = setup_test();

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(0),
        mock_info("anyone", &[]),
        ExecuteMsg::Claim {
            claims: vec![],
        },
    )
    .unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(0),
        mock_info("owner", &[]),
        ExecuteMsg::Claim {
            claims: vec![],
        },
    )
    .unwrap_err();
    assert_eq!(res, ContractError::NoClaimsProvided {});

    let res = execute(
        deps.as_mut(),
        mock_env_at_timestamp(0),
        mock_info("owner", &[]),
        ExecuteMsg::Claim {
            claims: vec![
                ClaimType::Default("claim1".to_string()),
                ClaimType::Default("claim2".to_string()),
            ],
        },
    )
    .unwrap();
    assert_eq!(res.messages.len(), 2);

    assert_eq!(
        res.messages[0].msg,
        ClaimExecuteMsg::Claim {}.into_msg("claim1".to_string()).unwrap()
    );
    assert_eq!(
        res.messages[1].msg,
        ClaimExecuteMsg::Claim {}.into_msg("claim2".to_string()).unwrap()
    );

    assert_eq!(res.attributes, vec![attr("action", "erishub/exec_claim")]);
}
