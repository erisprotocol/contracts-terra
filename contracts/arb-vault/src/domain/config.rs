use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdError};
use eris::arb_vault::ExecuteMsg;
use itertools::Itertools;

use crate::{
    constants::MAX_UNBOND_TIME_S,
    error::{ContractError, ContractResult},
    extensions::{ConfigEx, UtilizationMethodEx},
    state::State,
};

pub fn execute_update_config(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult {
    match msg {
        ExecuteMsg::UpdateConfig {
            utilization_method,
            unbond_time_s,
            insert_lsd,
            disable_lsd,
            remove_lsd,
            force_remove_lsd,
            fee_config,
            remove_whitelist,
            set_whitelist,
        } => {
            let state = State::default();
            state.assert_owner(deps.storage, &info.sender)?;

            let mut config = state.config.load(deps.storage)?;

            let mut config_changed = false;
            if let Some(unbond_time_s) = unbond_time_s {
                if unbond_time_s > MAX_UNBOND_TIME_S {
                    return Err(ContractError::ConfigTooHigh("unbond_time_s".into()));
                }
                config.unbond_time_s = unbond_time_s;
                config_changed = true;
            }

            if let Some(utilization_method) = utilization_method {
                utilization_method.validate()?;
                config.utilization_method = utilization_method;
                config_changed = true;
            }

            if let Some(insert_lsd) = insert_lsd {
                let mut lsds = config.lsd_group(&env);
                let lsd = lsds.get_adapter_by_name(&insert_lsd.name);

                if lsd.is_err() {
                    config.lsds.push(insert_lsd.validate(deps.api)?);
                    config_changed = true;
                } else {
                    return Err(ContractError::AdapterNameDuplicate(insert_lsd.name));
                }
            } else if let Some(disable_lsd) = disable_lsd {
                let lsd = config.lsds.iter_mut().find(|a| a.name == disable_lsd);
                if let Some(lsd) = lsd {
                    lsd.disabled = true;
                    config_changed = true;
                } else {
                    Err(ContractError::AdapterNotFound(disable_lsd))?
                }
            } else if let Some(remove_lsd) = remove_lsd {
                let mut lsds = config.lsd_group(&env);
                let lsd = lsds.get_adapter_by_name(&remove_lsd)?;
                let balance = lsd.get_balance(&deps.as_ref(), &env.contract.address)?;

                // cannot remove adapter if any funds still sent through the adapter
                if !balance.unbonding.is_zero()
                    || !balance.withdrawable.is_zero()
                    || !balance.xbalance.is_zero()
                {
                    Err(ContractError::CannotRemoveAdapterThatHasFunds {})?;
                }

                config.lsds =
                    config.lsds.into_iter().filter(|lsd| lsd.name != remove_lsd).collect_vec();

                config_changed = true;
            } else if let Some(force_remove_lsd) = force_remove_lsd {
                config.lsds = config
                    .lsds
                    .into_iter()
                    .filter(|lsd| lsd.name != force_remove_lsd)
                    .collect_vec();

                config_changed = true;
            }

            if config_changed {
                // after the config change, it still needs to be able to query all assets.
                let mut lsds = config.lsd_group(&env);
                lsds.get_total_assets_err(deps.as_ref(), &env, &state, &config)?;

                state.config.save(deps.storage, &config)?;
            }

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
