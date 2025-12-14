use astroport::asset::{Asset, AssetInfo};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Addr, CosmosMsg, QuerierWrapper, StdError, StdResult, Uint128, WasmMsg,
};

use crate::adapters::{
    asset::{AssetEx, AssetInfoEx, AssetInfosEx},
    msgs_zapper::{self, PostActionCreate, SupportsSwapResponse},
};

#[cw_serde]
pub struct Zapper(pub Addr);

impl Zapper {
    pub fn zap(
        &self,
        into: AssetInfo,
        assets: Vec<AssetInfo>,
        min_received: Option<Uint128>,
        post_action: Option<PostActionCreate>,
    ) -> Result<CosmosMsg, StdError> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            msg: to_json_binary(&msgs_zapper::ExecuteMsg::Zap {
                into: into.to_new(),
                assets: assets.to_new(),
                min_received,
                post_action,
            })?,
            funds: vec![],
        }))
    }

    pub fn swap_msgs(
        &self,
        into: AssetInfo,
        assets: Vec<Asset>,
        min_received: Option<Uint128>,
        receiver: Option<String>,
    ) -> Result<Vec<CosmosMsg>, StdError> {
        let mut funds = vec![];
        let mut msgs = vec![];
        let mut infos = vec![];

        for asset in assets {
            if asset.amount.is_zero() {
                continue;
            }
            match asset.info {
                AssetInfo::NativeToken {
                    ..
                } => funds.push(asset.to_coin()?),
                AssetInfo::Token {
                    ..
                } => msgs.push(asset.transfer_msg(&self.0.clone())?),
            }
            infos.push(asset.info);
        }

        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            msg: to_json_binary(&msgs_zapper::ExecuteMsg::Swap {
                into: into.to_new(),
                assets: infos.to_new(),
                min_received,
                receiver,
            })?,
            funds,
        }));

        Ok(msgs)
    }

    pub fn query_support_swap(
        &self,
        querier: &QuerierWrapper,
        from: AssetInfo,
        to: AssetInfo,
    ) -> StdResult<bool> {
        let res: SupportsSwapResponse = querier.query_wasm_smart(
            self.0.to_string(),
            &msgs_zapper::QueryMsg::SupportsSwap {
                from: from.to_new(),
                to: to.to_new(),
            },
        )?;
        Ok(res.suppored)
    }
}
