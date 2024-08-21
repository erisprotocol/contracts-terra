use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, StdResult, WasmMsg};
use cw_asset::AssetInfo;

#[cw_serde]
pub enum ExecuteMsg {
    ClaimRewards(AssetInfo),
}

pub struct Alliance(pub Addr);

impl Alliance {
    pub fn claim_msgs(
        &self,
        assets: Vec<astroport::asset::AssetInfo>,
    ) -> StdResult<Vec<CosmosMsg>> {
        let mut msgs = vec![];

        for asset in assets {
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: self.0.to_string(),
                msg: to_json_binary(&ExecuteMsg::ClaimRewards(match asset {
                    astroport::asset::AssetInfo::NativeToken {
                        denom,
                    } => cw_asset::AssetInfo::Native(denom),
                    astroport::asset::AssetInfo::Token {
                        contract_addr,
                    } => cw_asset::AssetInfo::Cw20(contract_addr),
                }))?,
                funds: vec![],
            }))
        }

        Ok(msgs)
    }
}
