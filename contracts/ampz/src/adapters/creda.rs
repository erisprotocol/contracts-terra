use astroport::asset::Asset;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, CosmosMsg, StdResult};
use eris::adapters::asset::AssetEx;

#[cw_serde]
pub enum ExecuteMsg {
    Supply {
        recipient: Option<String>,
        collateral: Option<bool>,
    },
    Repay {
        recipient: Option<String>,
    },
}

pub struct CredaPortfolio(pub Addr);

impl CredaPortfolio {
    pub fn deposit(&self, asset: Asset, recipient: Option<String>) -> StdResult<CosmosMsg> {
        let msg = asset.send_or_execute_msg(
            &self.0,
            &ExecuteMsg::Supply {
                recipient,
                collateral: None,
            },
        )?;

        Ok(msg)
    }

    pub fn repay(self, asset: Asset, recipient: Option<String>) -> StdResult<CosmosMsg> {
        let msg = asset.send_or_execute_msg(
            &self.0,
            &ExecuteMsg::Repay {
                recipient,
            },
        )?;

        Ok(msg)
    }
}
