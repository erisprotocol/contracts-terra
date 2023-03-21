use astroport::asset::Asset;
use cosmwasm_std::{coins, to_binary, Addr, CosmosMsg, Env, QuerierWrapper, StdResult, WasmMsg};
use cw20::Expiration;
use eris::adapters::asset::AssetEx;

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

pub struct CapapultOverseer(pub Addr);

impl CapapultOverseer {
    pub fn lock_collateral(&self, env: &Env, asset: Asset) -> StdResult<Vec<CosmosMsg>> {
        match &asset.info {
            astroport::asset::AssetInfo::Token {
                ..
            } => {
                let increase_allowance_msg = asset.increase_allowance_msg(
                    self.0.to_string(),
                    Some(Expiration::AtHeight(env.block.height + 1)),
                )?;

                let asset_info_str = asset.info.to_string();

                let lock_collateral_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: self.0.to_string(),
                    msg: to_binary(&capapult::overseer::ExecuteMsg::LockCollateral {
                        collaterals: vec![(asset_info_str, asset.amount.into())],
                    })?,
                    funds: vec![],
                });
                Ok(vec![increase_allowance_msg, lock_collateral_msg])
            },
            astroport::asset::AssetInfo::NativeToken {
                ..
            } => {
                let asset_info_str = asset.info.to_string();
                let funds = coins(asset.amount.u128(), asset_info_str.clone());

                let lock_collateral_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: self.0.to_string(),
                    msg: to_binary(&capapult::overseer::ExecuteMsg::LockCollateral {
                        collaterals: vec![(asset_info_str, asset.amount.into())],
                    })?,
                    funds,
                });
                Ok(vec![lock_collateral_msg])
            },
        }
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
