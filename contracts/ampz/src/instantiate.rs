use cosmwasm_std::{DepsMut, Env, Response, StdResult};
use cw2::set_contract_version;
use eris::{
    adapters::{arb_vault::ArbVault, farm::Farm, hub::Hub, zapper::Zapper},
    ampz::InstantiateMsg,
    helper::dedupe,
};

use crate::{
    constants::{CONTRACT_NAME, CONTRACT_VERSION},
    state::State,
};

pub fn exec_instantiate(deps: DepsMut, _env: Env, msg: InstantiateMsg) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let state = State::default();

    state.controller.save(deps.storage, &deps.api.addr_validate(&msg.controller)?)?;
    state.zapperv2.save(deps.storage, &Zapper(deps.api.addr_validate(&msg.zapper)?))?;
    state.astroport.save(deps.storage, &msg.astroport.validate(deps.api)?)?;
    state.capapult.save(deps.storage, &msg.capapult.validate(deps.api)?)?;

    state.owner.save(deps.storage, &deps.api.addr_validate(&msg.owner)?)?;
    state.hub.save(deps.storage, &Hub(deps.api.addr_validate(msg.hub.as_str())?))?;
    state
        .arb_vault
        .save(deps.storage, &ArbVault(deps.api.addr_validate(msg.arb_vault.as_str())?))?;

    let mut farms = msg.farms;
    dedupe(&mut farms);
    let farms: Vec<Farm> = farms
        .into_iter()
        .map(|a| Ok(Farm(deps.api.addr_validate(a.as_str())?)))
        .collect::<StdResult<_>>()?;

    state.farms.save(deps.storage, &farms)?;

    state.id.save(deps.storage, &1u128)?;
    state.fee.save(deps.storage, &msg.fee.validate(deps.api)?)?;

    state.tla.save(deps.storage, &msg.tla.validate(deps.api)?)?;
    state.creda.save(deps.storage, &msg.creda.validate(deps.api)?)?;
    state.whitewhale.save(deps.storage, &msg.whitewhale.validate(deps.api)?)?;
    state.alliance.save(deps.storage, &msg.alliance.validate(deps.api)?)?;

    Ok(Response::new().add_attribute("action", "ampz/exec_instantiate"))
}
