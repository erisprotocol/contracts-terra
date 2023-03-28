use cosmwasm_std::{Addr, Deps};
use eris::arb_vault::ClaimBalance;

use crate::error::{ContractError, CustomResult};

use super::lsdadapter::LsdAdapter;

pub struct LsdWrapper {
    pub adapter: Box<dyn LsdAdapter>,
    pub disabled: bool,
    pub name: String,
    pub wallet: Addr,
}

impl LsdWrapper {
    pub fn assert_not_disabled(&self) -> CustomResult<()> {
        if self.disabled {
            return Err(ContractError::AdapterDisabled(self.name.clone()));
        }

        Ok(())
    }

    pub fn get_balance(&mut self, deps: &Deps, addr: &Addr) -> CustomResult<ClaimBalance> {
        Ok(ClaimBalance {
            name: self.name.to_string(),
            withdrawable: self.adapter.query_withdrawable(deps)?,
            unbonding: self.adapter.query_unbonding(deps)?,
            xbalance: self.adapter.asset().query_pool(&deps.querier, addr)?,
            xfactor: self.adapter.query_factor_x_to_normal(deps)?,
        })
    }
}
