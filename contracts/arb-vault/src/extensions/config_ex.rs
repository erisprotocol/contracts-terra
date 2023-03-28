use crate::lsds::lsdgroup::LsdGroup;
use astroport::{asset::native_asset_info, querier::query_supply};
use cosmwasm_std::{Addr, Env, QuerierWrapper, StdResult, Uint128};
use eris::arb_vault::Config;
use itertools::Itertools;

pub trait ConfigEx {
    fn lsd_group(&self, env: &Env) -> LsdGroup;
    fn lsd_group_by_names(&self, env: &Env, names: Option<Vec<String>>) -> LsdGroup;
    fn query_utoken_amount(&self, querier: &QuerierWrapper, env: &Env) -> StdResult<Uint128>;
    fn query_lp_supply(&self, querier: &QuerierWrapper) -> StdResult<Uint128>;
}

impl ConfigEx for Config<Addr> {
    fn lsd_group(&self, env: &Env) -> LsdGroup {
        self.lsd_group_by_names(env, None)
    }

    fn lsd_group_by_names(&self, env: &Env, names: Option<Vec<String>>) -> LsdGroup {
        let lsds = if let Some(names) = names {
            self.lsds.iter().filter(|lsd| names.contains(&lsd.name)).collect_vec()
        } else {
            self.lsds.iter().collect_vec()
        };

        LsdGroup::new(&lsds, env.contract.address.clone())
    }

    fn query_utoken_amount(&self, querier: &QuerierWrapper, env: &Env) -> StdResult<Uint128> {
        native_asset_info(self.utoken.clone()).query_pool(querier, env.contract.address.clone())
    }

    fn query_lp_supply(&self, querier: &QuerierWrapper) -> StdResult<Uint128> {
        query_supply(querier, self.lp_addr.clone())
    }
}
