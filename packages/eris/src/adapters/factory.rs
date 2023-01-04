use astroport::asset::{Asset, AssetInfo, PairInfo};
use cosmwasm_std::{Addr, CosmosMsg, Decimal, QuerierWrapper, StdResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::pair::Pair;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Factory(pub Addr);

impl Factory {
    pub fn create_swap(
        &self,
        querier: &QuerierWrapper,
        offer_asset: &Asset,
        wanted: &AssetInfo,
        max_spread: Decimal,
        to: Option<String>,
    ) -> StdResult<CosmosMsg> {
        let pair_info = self.get_pair(querier, offer_asset, wanted)?;
        Pair(pair_info.contract_addr).swap_msg(offer_asset, None, Some(max_spread), to)
    }

    pub fn simulate(
        &self,
        querier: &QuerierWrapper,
        offer_asset: &Asset,
        wanted: &AssetInfo,
    ) -> StdResult<Asset> {
        let pair_info = self.get_pair(querier, offer_asset, wanted)?;
        Pair(pair_info.contract_addr.clone()).simulate_to_asset(querier, &pair_info, offer_asset)
    }

    fn get_pair(
        &self,
        querier: &QuerierWrapper,
        offer_asset: &Asset,
        wanted: &AssetInfo,
    ) -> StdResult<PairInfo> {
        let pair_info: PairInfo = querier.query_wasm_smart(
            self.0.to_string(),
            &astroport::factory::QueryMsg::Pair {
                asset_infos: vec![offer_asset.info.clone(), wanted.clone()],
            },
        )?;
        Ok(pair_info)
    }
}
