use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response, StdError, StdResult};
use eris::arb_vault::{ExecuteMsg, LsdConfig, ValidatedConfig};

use crate::{
    error::{ContractError, ContractResult, CustomResult},
    state::State,
};

pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult {
    match msg {
        ExecuteMsg::UpdateConfig {
            utilization_method,
            unbond_time_s,
            lsds: update_lsds,
            fee_config,
            remove_whitelist,
            set_whitelist,
        } => {
            let state = State::default();
            state.assert_owner(deps.storage, &info.sender)?;

            let api = deps.api;

            state.config.update(deps.storage, |mut config| -> CustomResult<ValidatedConfig> {
                if let Some(unbond_time_s) = unbond_time_s {
                    if unbond_time_s > 100 * 24 * 60 * 60 {
                        return Err(ContractError::UnbondTimeTooHigh);
                    }
                    config.unbond_time_s = unbond_time_s;
                }

                if let Some(utilization_method) = utilization_method {
                    // TODO validate input
                    config.utilization_method = utilization_method;
                }

                if let Some(update_lsds) = update_lsds {
                    // TODO validate input
                    config.lsds = update_lsds
                        .into_iter()
                        .map(|lsd| lsd.validate(api))
                        .collect::<StdResult<Vec<LsdConfig<Addr>>>>()?;
                }

                Ok(config)
            })?;

            if let Some(fee_config) = fee_config {
                state.fee_config.save(deps.storage, &fee_config.validate(deps.api)?)?;
            }

            if let Some(set_whitelist) = set_whitelist {
                state.update_whitelist(deps.storage, deps.api, set_whitelist)?;

                if remove_whitelist.is_some() {
                    Err(ContractError::CannotRemoveWhitelistWhileSettingIt {})?;
                }
            }

            if let Some(remove_whitelist) = remove_whitelist {
                if remove_whitelist {
                    state.whitelisted_addrs.remove(deps.storage);
                }
            }

            Ok(Response::new().add_attribute("action", "update_config"))
        },
        _ => Err(StdError::generic_err("not supported").into()),
    }
}
