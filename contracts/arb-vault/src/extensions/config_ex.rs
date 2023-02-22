use crate::lsds::{
    eris::Eris, lsdadapter::LsdAdapter, lsdgroup::LsdGroup, prism::Prism, stader::Stader,
    steak::Steak,
};
use astroport::{asset::native_asset_info, querier::query_supply};
use cosmwasm_std::{Addr, Env, QuerierWrapper, StdResult, Uint128};
use eris::arb_vault::{Config, LsdType};
use itertools::Itertools;

pub trait ConfigEx {
    fn lsd_group(&self, env: &Env) -> LsdGroup;
    fn lsd_group_wallet(&self, env: &Env, wallet: Option<Addr>) -> LsdGroup;
    fn query_utoken_amount(&self, querier: &QuerierWrapper, env: &Env) -> StdResult<Uint128>;
    fn query_lp_supply(&self, querier: &QuerierWrapper) -> StdResult<Uint128>;
}

impl ConfigEx for Config<Addr> {
    fn lsd_group(&self, env: &Env) -> LsdGroup {
        self.lsd_group_wallet(env, None)
    }

    fn lsd_group_wallet(&self, env: &Env, wallet: Option<Addr>) -> LsdGroup {
        // this allows to see any unbonding state for any address in the terra network and can be used by the query interface.
        let wallet_address = wallet.unwrap_or_else(|| env.contract.address.clone());

        LsdGroup {
            lsd_adapters: self
                .lsds
                .iter()
                .map(|lsd| -> Box<dyn LsdAdapter> {
                    match lsd.lsd_type.clone() {
                        LsdType::Eris {
                            addr,
                            cw20,
                        } => Box::new(Eris {
                            state_cache: None,
                            undelegation_records_cache: None,
                            addr,
                            cw20,
                            wallet: wallet_address.clone(),
                        }),
                        LsdType::Backbone {
                            addr,
                            cw20,
                        } => Box::new(Steak {
                            state_cache: None,
                            undelegation_records_cache: None,
                            addr,
                            cw20,
                            wallet: wallet_address.clone(),
                        }),
                        LsdType::Stader {
                            addr,
                            cw20,
                        } => Box::new(Stader {
                            state_cache: None,
                            undelegation_records_cache: None,
                            addr,
                            cw20,
                            wallet: wallet_address.clone(),
                        }),
                        LsdType::Prism {
                            addr,
                            cw20,
                        } => Box::new(Prism {
                            state_cache: None,
                            unbonding_cache: None,
                            addr,
                            cw20,
                            wallet: wallet_address.clone(),
                        }),
                    }
                })
                .collect_vec(),
        }
    }

    fn query_utoken_amount(&self, querier: &QuerierWrapper, env: &Env) -> StdResult<Uint128> {
        native_asset_info(self.utoken.clone()).query_pool(querier, env.contract.address.clone())
    }

    fn query_lp_supply(&self, querier: &QuerierWrapper) -> StdResult<Uint128> {
        query_supply(querier, self.lp_addr.clone())
    }
}
