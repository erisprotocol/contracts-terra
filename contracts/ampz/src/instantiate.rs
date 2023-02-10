use cosmwasm_std::{DepsMut, Env, Response, StdResult};
use cw2::set_contract_version;
use eris::{
    adapters::{compounder::Compounder, farm::Farm, hub::Hub},
    ampz::InstantiateMsg,
};

use crate::{
    constants::{CONTRACT_NAME, CONTRACT_VERSION},
    state::State,
};

pub fn exec_instantiate(deps: DepsMut, _env: Env, msg: InstantiateMsg) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let state = State::default();

    state.controller.save(deps.storage, &deps.api.addr_validate(&msg.controller)?)?;
    state.zapper.save(deps.storage, &Compounder(deps.api.addr_validate(&msg.zapper)?))?;
    state.astroport.save(deps.storage, &msg.astroport.validate(deps.api)?)?;

    state.owner.save(deps.storage, &deps.api.addr_validate(&msg.owner)?)?;
    state.hub.save(deps.storage, &Hub(deps.api.addr_validate(msg.hub.as_str())?))?;

    let farms: Vec<Farm> = msg
        .farms
        .into_iter()
        .map(|a| Ok(Farm(deps.api.addr_validate(a.as_str())?)))
        .collect::<StdResult<_>>()?;

    state.farms.save(deps.storage, &farms)?;

    state.id.save(deps.storage, &1u128)?;
    state.fee.save(deps.storage, &msg.fee.validate(deps.api)?)?;

    Ok(Response::new())
}
