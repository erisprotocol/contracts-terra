use crate::error::{ContractError, CustomResult};
use cosmwasm_std::{Decimal, Uint128};
use eris::arb_vault::{BalancesDetails, ClaimBalance, UtilizationMethod, ValidatedConfig};

pub trait BalancesEx {
    fn get_max_utilization_for_profit(
        &self,
        config: &ValidatedConfig,
        profit: &Decimal,
    ) -> CustomResult<Decimal>;

    fn calc_all_takeable_steps(
        &self,
        config: &ValidatedConfig,
    ) -> CustomResult<Vec<(Decimal, Uint128)>>;

    fn calc_takeable_for_profit(
        &self,
        config: &ValidatedConfig,
        profit: &Decimal,
    ) -> CustomResult<Uint128>;

    fn get_by_name(&self, name: &str) -> CustomResult<&ClaimBalance>;
}

impl BalancesEx for BalancesDetails {
    fn get_max_utilization_for_profit(
        &self,
        config: &ValidatedConfig,
        profit: &Decimal,
    ) -> CustomResult<Decimal> {
        match config.utilization_method.clone() {
            UtilizationMethod::Steps(steps) => {
                let step = steps
                    .into_iter()
                    .find(|step| step.0.eq(profit))
                    .ok_or(ContractError::NotSupportedProfitStep(*profit))?;

                Ok(step.1)
            },
        }
    }

    fn calc_all_takeable_steps(
        &self,
        config: &ValidatedConfig,
    ) -> CustomResult<Vec<(Decimal, Uint128)>> {
        match config.utilization_method.clone() {
            UtilizationMethod::Steps(steps) => steps
                .into_iter()
                .map(|step| {
                    let max_utilization = step.1;
                    let vault_takeable = calc_vault_takeable(
                        max_utilization,
                        self.vault_total,
                        self.vault_takeable,
                    )?;

                    Ok((step.0, vault_takeable))
                })
                .collect::<CustomResult<Vec<(Decimal, Uint128)>>>(),
        }
    }

    fn calc_takeable_for_profit(
        &self,
        config: &ValidatedConfig,
        profit: &Decimal,
    ) -> CustomResult<Uint128> {
        // same calculation as above
        let max_utilization = self.get_max_utilization_for_profit(config, profit)?;
        let vault_takeable =
            calc_vault_takeable(max_utilization, self.vault_total, self.vault_takeable)?;

        Ok(vault_takeable)
    }

    fn get_by_name(&self, name: &str) -> CustomResult<&ClaimBalance> {
        if let Some(claim) = self.details.iter().find(|detail| detail.name == *name) {
            Ok(claim)
        } else {
            Err(ContractError::AdapterNotFound(name.into()))
        }
    }
}

fn calc_vault_takeable(
    max_utilization: Decimal,
    vault_total: Uint128,
    vault_takeable: Uint128,
) -> CustomResult<Uint128> {
    // 0.5% arbitrage gain, utilize up to 20% of the pool size
    // 1000 vault_total
    //  100 takeable -> 1000 * 20% + 100 - 1000 = 0
    //  900 takeable -> 1000 * 20% + 900 - 1000 = 100
    Ok((vault_total * max_utilization)
        .checked_add(vault_takeable)?
        .checked_sub(vault_total)
        .unwrap_or_default())
}
