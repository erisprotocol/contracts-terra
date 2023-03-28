use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdError, StdResult};
use eris::{
    adapters::{compounder::Compounder, farm::Farm, hub::Hub},
    ampz::ExecuteMsg,
};
use itertools::Itertools;

use crate::{
    error::{ContractError, ContractResult},
    state::State,
};

pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult {
    match msg {
        ExecuteMsg::UpdateConfig {
            add_farms,
            remove_farms,
            controller,
            astroport,
            zapper,

            fee,
            hub,
            capapult,
        } => {
            let state = State::default();
            state.assert_owner(deps.storage, &info.sender)?;

            if let Some(add_farms) = add_farms {
                if remove_farms.is_some() {
                    return Err(ContractError::CannotAddAndRemoveFarms {});
                }

                let add_farms: Vec<Farm> = add_farms
                    .into_iter()
                    .map(|a| Ok(Farm(deps.api.addr_validate(a.as_str())?)))
                    .collect::<StdResult<_>>()?;

                state.farms.update::<_, StdError>(deps.storage, |mut farms| {
                    for farm in add_farms {
                        if !farms.contains(&farm) {
                            farms.push(farm);
                        }
                    }

                    Ok(farms)
                })?;
            }

            if let Some(remove_farms) = remove_farms {
                let remove_farms: Vec<Farm> = remove_farms
                    .into_iter()
                    .map(|a| Ok(Farm(deps.api.addr_validate(a.as_str())?)))
                    .collect::<StdResult<_>>()?;

                state.farms.update::<_, StdError>(deps.storage, |farms| {
                    let farms =
                        farms.into_iter().filter(|farm| !remove_farms.contains(farm)).collect_vec();

                    Ok(farms)
                })?;
            }

            if let Some(controller) = controller {
                state.controller.save(deps.storage, &deps.api.addr_validate(&controller)?)?;
            }

            if let Some(astroport) = astroport {
                state.astroport.save(deps.storage, &astroport.validate(deps.api)?)?;
            }

            if let Some(capapult) = capapult {
                state.capapult.save(deps.storage, &capapult.validate(deps.api)?)?;
            }

            if let Some(zapper) = zapper {
                state.zapper.save(deps.storage, &Compounder(deps.api.addr_validate(&zapper)?))?;
            }
            if let Some(hub) = hub {
                state.hub.save(deps.storage, &Hub(deps.api.addr_validate(&hub)?))?;
            }

            if let Some(fee) = fee {
                state.fee.save(deps.storage, &fee.validate(deps.api)?)?;
            }

            Ok(Response::new().add_attribute("action", "ampz/update_config"))
        },
        _ => Err(ContractError::NotSupported {}),
    }
}
