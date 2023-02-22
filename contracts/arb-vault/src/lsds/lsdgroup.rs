use astroport::asset::AssetInfo;
use cosmwasm_std::{attr, Attribute, CosmosMsg, Deps, DepsMut, Env, Uint128};
use eris::arb_vault::{Balances, ClaimBalance, ValidatedConfig};

use crate::{
    error::{ContractError, CustomResult},
    extensions::config_ex::ConfigEx,
    state::State,
};

use super::lsdadapter::LsdAdapter;

pub struct LsdGroup {
    pub lsd_adapters: Vec<Box<dyn LsdAdapter>>,
}

impl LsdGroup {
    pub fn get(&mut self, asset_info: AssetInfo) -> CustomResult<&mut Box<dyn LsdAdapter>> {
        let result = self.lsd_adapters.iter_mut().find(|t| t.asset() == asset_info);
        result.ok_or(ContractError::AssetUnknown {})
    }

    pub fn get_unbonding(&mut self, deps: &Deps) -> CustomResult<Uint128> {
        self.lsd_adapters.iter_mut().map(|a| a.query_unbonding(deps)).sum()
    }

    pub fn get_withdrawable(&mut self, deps: &Deps) -> CustomResult<Uint128> {
        self.lsd_adapters.iter_mut().map(|a| a.query_withdrawable(deps)).sum()
    }

    pub fn get_balances(&mut self, deps: &Deps) -> CustomResult<Vec<ClaimBalance>> {
        self.lsd_adapters
            .iter_mut()
            .map(|c| {
                Ok(ClaimBalance {
                    withdrawable: c.query_withdrawable(deps)?,
                    unbonding: c.query_unbonding(deps)?,
                })
            })
            .collect::<CustomResult<Vec<ClaimBalance>>>()
    }

    pub fn get_withdraw_msgs(
        &mut self,
        deps: &DepsMut,
    ) -> CustomResult<(Vec<CosmosMsg>, Vec<Attribute>)> {
        let mut messages: Vec<CosmosMsg> = vec![];
        let mut attributes: Vec<Attribute> = vec![attr("action", "arb/execute_withdraw_liquidity")];

        for claim in self.lsd_adapters.iter_mut() {
            let claimable_amount = claim.query_withdrawable(&deps.as_ref())?;

            if !claimable_amount.is_zero() {
                let mut msgs = claim.withdraw(&deps.as_ref(), claimable_amount)?;
                messages.append(&mut msgs);
                attributes.push(attr("type", claim.get_name()));
                attributes.push(attr("withdraw_amount", claimable_amount))
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
    ) -> CustomResult<Balances> {
        self.get_total_assets(deps, env, state, config)
            .map_err(|e| ContractError::CouldNotLoadTotalAssets(e.to_string()))
    }

    pub(crate) fn get_total_assets(
        &mut self,
        deps: Deps,
        env: &Env,
        state: &State,
        config: &ValidatedConfig,
    ) -> CustomResult<Balances> {
        let vault_available = config.query_utoken_amount(&deps.querier, env)?;

        let locked_user_withdrawls = state.balance_locked.load(deps.storage)?.balance;
        let lsd_unbonding = self.get_unbonding(&deps)?;
        let lsd_withdrawable = self.get_withdrawable(&deps)?;

        // tvl_utoken = available + unbonding + withdrawable
        let tvl_utoken =
            vault_available.checked_add(lsd_unbonding)?.checked_add(lsd_withdrawable)?;

        Ok(Balances {
            tvl_utoken,
            lsd_unbonding,
            lsd_withdrawable,
            vault_total: tvl_utoken.checked_sub(locked_user_withdrawls).unwrap_or_default(),
            vault_available,
            vault_takeable: vault_available.checked_sub(locked_user_withdrawls).unwrap_or_default(),
            locked_user_withdrawls,
        })
    }
}
