use astroport::asset::AssetInfo;
use astroport::generator::UserInfoV2;
use astroport::restricted_vector::RestrictedVector;
use cosmwasm_std::{Addr, QuerierWrapper, StdResult};
use cw_storage_plus::Map;
use eris::adapters::generator::Generator;

const USER_INFO: Map<(&Addr, &Addr), UserInfoV2> = Map::new("user_info");
const PROXY_REWARD_ASSET: Map<&Addr, AssetInfo> = Map::new("proxy_reward_asset");

pub trait GeneratorEx {
    fn query_user_info(
        &self,
        querier: &QuerierWrapper,
        lp_token: &Addr,
        user: &Addr,
    ) -> StdResult<Option<UserInfoV2>>;
    fn query_proxy_reward_asset(
        &self,
        querier: &QuerierWrapper,
        proxy_addr: &Addr,
    ) -> StdResult<Option<AssetInfo>>;
}

impl GeneratorEx for Generator {
    fn query_user_info(
        &self,
        querier: &QuerierWrapper,
        lp_token: &Addr,
        user: &Addr,
    ) -> StdResult<Option<UserInfoV2>> {
        let op = USER_INFO.query(querier, self.0.clone(), (lp_token, user))?;
        let result = match op {
            Some(mut user_info) if !user_info.reward_debt_proxy.is_empty() => {
                let mut reward_debt_proxy = RestrictedVector::default();
                for (proxy_addr, value) in user_info.reward_debt_proxy.inner_ref() {
                    if let Some(asset_info) = self.query_proxy_reward_asset(querier, proxy_addr)? {
                        let token = match asset_info {
                            AssetInfo::Token {
                                contract_addr,
                            } => contract_addr,
                            AssetInfo::NativeToken {
                                denom,
                            } => Addr::unchecked(denom),
                        };
                        reward_debt_proxy.update(&token, *value)?;
                    }
                }
                user_info.reward_debt_proxy = reward_debt_proxy;
                Some(user_info)
            },
            it => it,
        };
        Ok(result)
    }

    fn query_proxy_reward_asset(
        &self,
        querier: &QuerierWrapper,
        proxy_addr: &Addr,
    ) -> StdResult<Option<AssetInfo>> {
        PROXY_REWARD_ASSET.query(querier, self.0.clone(), proxy_addr)
    }
}
