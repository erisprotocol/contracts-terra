pub mod balances_ex;
pub mod config_ex;

use cosmwasm_std::Decimal;
use eris::arb_vault::UtilizationMethod;

use crate::error::{ContractError, CustomResult};

pub use self::balances_ex::BalancesEx;
pub use self::config_ex::ConfigEx;

pub trait UtilizationMethodEx {
    fn validate(&self) -> CustomResult<()>;
}

impl UtilizationMethodEx for UtilizationMethod {
    fn validate(&self) -> CustomResult<()> {
        match &self {
            UtilizationMethod::Steps(steps) => {
                for step in steps {
                    if step.0 < Decimal::permille(5) {
                        // less than 0.5 % profit not allowed
                        return Err(ContractError::ConfigTooHigh("step min profit".into()));
                    }

                    if step.1 > Decimal::one() {
                        return Err(ContractError::ConfigTooHigh("step max take".into()));
                    }
                }
            },
        }

        Ok(())
    }
}
