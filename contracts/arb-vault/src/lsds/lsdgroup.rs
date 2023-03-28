use astroport::asset::AssetInfo;
use cosmwasm_std::{attr, Addr, Attribute, CosmosMsg, Deps, DepsMut, Env, Uint128};
use eris::arb_vault::{BalancesDetails, ClaimBalance, LsdConfig, LsdType, ValidatedConfig};
use itertools::Itertools;

use crate::{
    error::{ContractError, CustomResult},
    extensions::config_ex::ConfigEx,
    state::State,
};

use super::{eris::Eris, lsdwrapper::LsdWrapper, prism::Prism, stader::Stader, steak::Steak};

pub struct LsdGroup {
    lsds: Vec<LsdWrapper>,
}

impl LsdGroup {
    pub fn new(lsd_configs: &[&LsdConfig<Addr>], wallet_address: Addr) -> LsdGroup {
        let lsds = lsd_configs
            .iter()
            .map(|config| -> LsdWrapper {
                LsdWrapper {
                    disabled: config.disabled,
                    name: config.name.clone(),
                    wallet: wallet_address.clone(),
                    adapter: match config.lsd_type.clone() {
                        LsdType::Eris {
                            addr,
                            cw20,
                        } => Box::new(Eris {
                            state_cache: None,
                            undelegation_records_cache: None,
                            addr,
                            cw20,
                            wallet: wallet_address.clone(),
                        }),
                        LsdType::Backbone {
                            addr,
                            cw20,
                        } => Box::new(Steak {
                            state_cache: None,
                            undelegation_records_cache: None,
                            addr,
                            cw20,
                            wallet: wallet_address.clone(),
                        }),
                        LsdType::Stader {
                            addr,
                            cw20,
                        } => Box::new(Stader {
                            state_cache: None,
                            undelegation_records_cache: None,
                            addr,
                            cw20,
                            wallet: wallet_address.clone(),
                        }),
                        LsdType::Prism {
                            addr,
                            cw20,
                        } => Box::new(Prism {
                            state_cache: None,
                            unbonding_cache: None,
                            addr,
                            cw20,
                            wallet: wallet_address.clone(),
                        }),
                    },
                }
            })
            .collect_vec();

        LsdGroup {
            lsds,
        }
    }

    pub fn get_adapter_by_asset(&mut self, asset_info: AssetInfo) -> CustomResult<&mut LsdWrapper> {
        let result = self.lsds.iter_mut().find(|t| t.adapter.asset() == asset_info);
        result.ok_or_else(|| ContractError::AdapterNotFound(format!("token - {0}", asset_info)))
    }

    pub fn get_adapter_by_name(&mut self, name: &String) -> CustomResult<&mut LsdWrapper> {
        let result = self.lsds.iter_mut().find(|t| t.name == *name);
        result.ok_or_else(|| ContractError::AdapterNotFound(name.clone()))
    }

    // pub fn get_unbonding(&mut self, deps: &Deps) -> CustomResult<Uint128> {
    //     self.lsds.iter_mut().map(|a| a.adapter.query_unbonding(deps)).sum()
    // }

    // pub fn get_withdrawable(&mut self, deps: &Deps) -> CustomResult<Uint128> {
    //     self.lsds.iter_mut().map(|a| a.adapter.query_withdrawable(deps)).sum()
    // }

    pub fn get_balances(&mut self, deps: &Deps, addr: &Addr) -> CustomResult<Vec<ClaimBalance>> {
        self.lsds
            .iter_mut()
            .map(|c| c.get_balance(deps, addr))
            .collect::<CustomResult<Vec<ClaimBalance>>>()
    }

    pub fn get_withdraw_msgs(
        &mut self,
        deps: &DepsMut,
    ) -> CustomResult<(Vec<CosmosMsg>, Vec<Attribute>)> {
        let mut messages: Vec<CosmosMsg> = vec![];
        let mut attributes: Vec<Attribute> = vec![attr("action", "arb/execute_withdraw_liquidity")];

        for lsd in self.lsds.iter_mut() {
            let claimable_amount = lsd.adapter.query_withdrawable(&deps.as_ref())?;

            if !claimable_amount.is_zero() {
                let mut msgs = lsd.adapter.withdraw(&deps.as_ref(), claimable_amount)?;
                messages.append(&mut msgs);
                attributes.push(attr("type", lsd.name.clone()));
                attributes.push(attr("withdraw_amount", claimable_amount))
            }
        }
        Ok((messages, attributes))
    }

    pub fn get_unbond_msgs(
        &mut self,
        deps: &DepsMut,
    ) -> CustomResult<(Vec<CosmosMsg>, Vec<Attribute>)> {
        let mut messages: Vec<CosmosMsg> = vec![];
        let mut attributes: Vec<Attribute> = vec![attr("action", "arb/execute_unbond_liquidity")];

        for lsd in self.lsds.iter_mut() {
            let unbondable_amount = lsd.adapter.asset().query_pool(&deps.querier, &lsd.wallet)?;

            if !unbondable_amount.is_zero() {
                let mut msgs = lsd.adapter.unbond(&deps.as_ref(), unbondable_amount)?;
                messages.append(&mut msgs);
                attributes.push(attr("type", lsd.name.clone()));
                attributes.push(attr("unbond_amount", unbondable_amount))
            }
        }
        Ok((messages, attributes))
    }

    pub(crate) fn get_total_assets_err(
        &mut self,
        deps: Deps,
        env: &Env,
        state: &State,
        config: &ValidatedConfig,
    ) -> CustomResult<BalancesDetails> {
        self.get_total_assets(deps, env, state, config)
            .map_err(|e| ContractError::CouldNotLoadTotalAssets(e.to_string()))
    }

    fn get_total_assets(
        &mut self,
        deps: Deps,
        env: &Env,
        state: &State,
        config: &ValidatedConfig,
    ) -> CustomResult<BalancesDetails> {
        let vault_available = config.query_utoken_amount(&deps.querier, env)?;

        let locked_user_withdrawls = state.balance_locked.load(deps.storage)?.balance;
        let balances = self.get_balances(&deps, &env.contract.address)?;

        let mut lsd_unbonding = Uint128::zero();
        let mut lsd_withdrawable = Uint128::zero();
        let mut lsd_xvalue = Uint128::zero();

        for balance in balances.iter() {
            lsd_unbonding += balance.unbonding;
            lsd_withdrawable += balance.withdrawable;
            lsd_xvalue += balance.xbalance * balance.xfactor;
        }

        // tvl_utoken = available + unbonding + withdrawable
        let tvl_utoken = vault_available
            .checked_add(lsd_unbonding)?
            .checked_add(lsd_withdrawable)?
            .checked_add(lsd_xvalue)?;

        Ok(BalancesDetails {
            tvl_utoken,
            lsd_unbonding,
            lsd_withdrawable,
            lsd_xvalue,
            vault_total: tvl_utoken.checked_sub(locked_user_withdrawls).unwrap_or_default(),
            vault_available,
            vault_takeable: vault_available.checked_sub(locked_user_withdrawls).unwrap_or_default(),
            locked_user_withdrawls,
            details: balances,
        })
    }

    pub(crate) fn assert_not_lsd_contract(&self, contract_addr: &Addr) -> CustomResult<()> {
        for lsd in self.lsds.iter() {
            // not allowed to call any used LSD contracts - additional safety measure
            if lsd.adapter.used_contracts().contains(contract_addr) {
                return Err(ContractError::CannotCallLsdContract {});
            }
        }

        Ok(())
    }
}
