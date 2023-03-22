use astroport::asset::Asset;
use cosmwasm_std::{to_binary, Addr, CosmosMsg, QuerierWrapper, StdResult, WasmMsg};
use eris::adapters::asset::AssetEx;

use crate::error::{ContractError, CustomResult};

pub struct CapapultMarket(pub Addr);

impl CapapultMarket {
    pub fn query_borrower_info(
        &self,
        querier: &QuerierWrapper,
        borrower: impl Into<String>,
    ) -> StdResult<capapult::market::BorrowerInfoResponse> {
        querier.query_wasm_smart(
            self.0.to_string(),
            &capapult::market::QueryMsg::BorrowerInfo {
                borrower: borrower.into(),
            },
        )
    }

    pub fn repay_loan(&self, asset: Asset) -> StdResult<CosmosMsg> {
        let msg = asset.transfer_msg_target(
            &self.0,
            Some(to_binary(&capapult::market::Cw20HookMsg::RepayStable {})?),
        )?;

        Ok(msg)
    }
}

pub struct CapapultLocker {
    pub overseer: Addr,
    pub custody: Addr,
}

impl CapapultLocker {
    pub fn deposit_and_lock_collateral(&self, asset: Asset) -> CustomResult<Vec<CosmosMsg>> {
        if asset.info.is_native_token() {
            return Err(ContractError::NotSupported {});
        }

        let deposit_collateral_msg = asset.transfer_msg_target(
            &self.custody,
            Some(to_binary(&capapult::custody::Cw20HookMsg::DepositCollateral {})?),
        )?;

        let asset_info_str = asset.info.to_string();
        let lock_collateral_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.overseer.to_string(),
            msg: to_binary(&capapult::overseer::ExecuteMsg::LockCollateral {
                collaterals: vec![(asset_info_str, asset.amount.into())],
            })?,
            funds: vec![],
        });

        Ok(vec![deposit_collateral_msg, lock_collateral_msg])
    }
}

#[cfg(test)]
mod test {
    use astroport::asset::token_asset_info;
    use cosmwasm_std::Addr;

    #[test]
    fn test_asset_info_to_string_right() {
        assert_eq!(token_asset_info(Addr::unchecked("hello")).to_string(), "hello".to_string());
    }
}
