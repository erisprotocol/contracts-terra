use astroport::asset::{Asset, AssetInfo};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, Coin, CosmosMsg, StdError, WasmMsg};
use eris::adapters::asset::AssetEx;

#[cw_serde]
pub enum ExecuteMsg {
    ClaimRewards {
        assets: Option<Vec<AssetInfo>>,
        recipient: Option<String>,
    },

    // user
    Stake {
        recipient: Option<String>,
    },
}

#[cw_serde]
pub struct TlaAssetStaking(pub Addr);

impl TlaAssetStaking {
    pub fn claim_rewards_msg(
        &self,
        assets: Option<Vec<AssetInfo>>,
        recipient: Option<String>,
    ) -> Result<CosmosMsg, StdError> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            msg: to_json_binary(&ExecuteMsg::ClaimRewards {
                assets,
                recipient,
            })?,
            funds: vec![],
        }))
    }

    pub fn stake_msg(
        &self,
        asset: Asset,
        recipient: Option<String>,
    ) -> Result<CosmosMsg, StdError> {
        asset.send_or_execute_msg(
            self.0.to_string(),
            &ExecuteMsg::Stake {
                recipient,
            },
        )
    }
}

#[cw_serde]
pub enum TlaConnectorExecuteMsg {
    Withdraw {
        recipient: Option<String>,
    },
}

#[cw_serde]
pub struct TlaConnector(pub Addr);

impl TlaConnector {
    pub fn withdraw_msg(
        &self,
        coin: Coin,
        recipient: Option<String>,
    ) -> Result<CosmosMsg, StdError> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            msg: to_json_binary(&TlaConnectorExecuteMsg::Withdraw {
                recipient,
            })?,
            funds: vec![coin],
        }))
    }
}
