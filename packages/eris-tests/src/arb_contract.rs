use astroport::asset::Asset;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    entry_point, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};
use eris::adapters::asset::AssetEx;

pub type ContractResult = Result<Response, StdError>;

#[cw_serde]
pub enum ExecuteMsg {
    ReturnAsset {
        asset: Asset,
        received: Vec<Coin>,
    },
}

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum QueryMsg {}

#[entry_point]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> ContractResult {
    Ok(Response::new())
}

#[entry_point]
pub fn execute(_deps: DepsMut, _env: Env, info: MessageInfo, msg: ExecuteMsg) -> ContractResult {
    match msg {
        ExecuteMsg::ReturnAsset {
            asset,
            received,
        } => {
            // contract validates that it received the expected funds and sends requested assets back to sender.
            assert_eq!(info.funds, received);
            let msg = asset.transfer_msg(&info.sender)?;
            Ok(Response::new().add_message(msg))
        },
    }
}

#[entry_point]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    Err(StdError::generic_err("not supported"))
}
