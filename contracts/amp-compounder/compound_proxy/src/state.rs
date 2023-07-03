use std::collections::HashSet;

use astroport::{
    asset::{Asset, AssetInfo, AssetInfoExt},
    common::OwnershipProposal,
};
use cosmwasm_std::{
    Addr, CosmosMsg, Decimal, DepsMut, QuerierWrapper, StdError, StdResult, Storage,
};
use cw_storage_plus::{Item, Map};
use eris::{
    adapters::{
        factory::Factory,
        pair::Pair,
        router::{Router, RouterType},
    },
    compound_proxy::{LpConfig, LpInit, PairInfo, PairType, RouteInit},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::ContractError;

/// This structure describes the main control config of pair.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    pub factory: Option<Factory>,
    pub owner: Addr,
}

impl Config {
    pub fn assert_owner(&self, sender: &Addr) -> StdResult<()> {
        if *sender == self.owner {
            Ok(())
        } else {
            Err(StdError::generic_err("unauthorized: sender is not owner"))
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RouteConfig {
    pub key: (AssetInfo, AssetInfo),
    pub route_type: RouteType,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum RouteType {
    Path {
        router: Router,
        router_type: RouterType,
        route: Vec<AssetInfo>,
    },
    PairProxy {
        pair_info: PairInfo,
    },
}

impl RouteConfig {
    pub fn create_swap(
        &self,
        offer_asset: &Asset,
        max_spread: Decimal,
        to: Option<Addr>,
    ) -> StdResult<CosmosMsg> {
        match &self.route_type {
            RouteType::Path {
                route,
                router,
                router_type,
            } => router.execute_swap_operations_msg(
                offer_asset.clone(),
                router_type.create_swap_operations(route)?,
                None,
                to,
                Some(max_spread),
            ),
            RouteType::PairProxy {
                pair_info,
            } => Pair(pair_info.contract_addr.clone()).swap_msg(
                offer_asset,
                None,
                Some(max_spread),
                to.map(|to| to.to_string()),
            ),
        }
    }

    pub fn simulate(&self, querier: &QuerierWrapper, offer_asset: &Asset) -> StdResult<Asset> {
        match &self.route_type {
            RouteType::Path {
                route,
                router,
                router_type,
            } => {
                let simulation = router.simulate(
                    querier,
                    offer_asset.amount,
                    router_type.create_swap_operations(route)?,
                )?;

                Ok(route[route.len() - 1].with_balance(simulation.amount))
            },
            RouteType::PairProxy {
                pair_info,
            } => Pair(pair_info.contract_addr.clone()).simulate_to_asset(
                querier,
                pair_info,
                offer_asset,
            ),
        }
    }
}

pub(crate) struct State<'a> {
    /// config
    pub config: Item<'a, Config>,
    /// ownership proposal
    pub ownership_proposal: Item<'a, OwnershipProposal>,
    /// lps allowed - indexed by LP Token
    pub lps: Map<'a, String, LpConfig>,
    /// routes usable from (start, end)
    pub routes: Map<'a, (&'a [u8], &'a [u8]), RouteConfig>,
    //// specifies the default allowed slippage - if unset use 10%
    pub default_max_spread: Item<'a, u64>,
}

impl Default for State<'static> {
    fn default() -> Self {
        Self {
            config: Item::new("config"),
            ownership_proposal: Item::new("ownership_proposal"),
            lps: Map::new("lps"),
            routes: Map::new("routes"),
            default_max_spread: Item::new("default_max_spread"),
        }
    }
}

impl<'a> State<'a> {
    pub fn assert_owner(&self, storage: &dyn Storage, sender: &Addr) -> StdResult<()> {
        self.config.load(storage)?.assert_owner(sender)?;
        Ok(())
    }

    pub fn get_default_max_spread(&self, storage: &dyn Storage) -> Decimal {
        // by default a max_spread of 10% is used.
        Decimal::percent(self.default_max_spread.load(storage).unwrap_or(10))
    }

    pub fn remove_lp(&self, deps: &mut DepsMut, lp_token: String) -> StdResult<()> {
        let pair_contract = deps.api.addr_validate(lp_token.as_str())?;

        if !self.lps.has(deps.storage, pair_contract.to_string()) {
            return Err(StdError::generic_err(format!("Lp {} not found", pair_contract)));
        }

        self.lps.remove(deps.storage, pair_contract.to_string());

        Ok(())
    }

    pub fn add_lp(&self, deps: &mut DepsMut, lp_init: LpInit) -> Result<(), ContractError> {
        let pair_contract = deps.api.addr_validate(lp_init.pair_contract.as_str())?;

        let pair_info: PairInfo = match lp_init.lp_type {
            Some(eris::compound_proxy::LpType::WhiteWhale) => {
                Pair(pair_contract).query_ww_pair_info(&deps.querier)?
            },
            _ => Pair(pair_contract).query_pair_info(&deps.querier)?,
        };

        if !pair_info.asset_infos.contains(&lp_init.wanted_token) {
            return Err(ContractError::WantedTokenNotInPair(lp_init.wanted_token.to_string()));
        }

        validate_slippage(lp_init.slippage_tolerance)?;

        match pair_info.pair_type {
            PairType::Xyk {} => (),
            PairType::Stable {} => (),
            PairType::XykWhiteWhale {} => (),
            _ => Err(StdError::generic_err("Custom pair type not supported"))?,
        }

        self.lps.save(
            deps.storage,
            pair_info.liquidity_token.to_string(),
            &LpConfig {
                pair_info,
                commission_bps: validate_commission(lp_init.commission_bps)?,
                slippage_tolerance: validate_percentage(
                    lp_init.slippage_tolerance,
                    "slippage_tolerance",
                )?,
                wanted_token: lp_init.wanted_token,
            },
        )?;

        Ok(())
    }

    pub fn delete_route(
        &self,
        deps: &mut DepsMut,
        assets: (AssetInfo, AssetInfo),
        both: bool,
    ) -> StdResult<()> {
        let key = (assets.0.as_bytes(), assets.1.as_bytes());

        if !self.routes.has(deps.storage, key) {
            return Err(StdError::generic_err("Route not found"));
        }

        self.routes.remove(deps.storage, key);
        if both {
            self.delete_route(deps, (assets.1, assets.0), false)?;
        }
        Ok(())
    }

    pub fn add_route(&self, deps: &mut DepsMut, route: RouteInit) -> StdResult<()> {
        match route {
            RouteInit::Path {
                route,
                router,
                router_type,
            } => {
                let mut set = HashSet::new();
                for segment in route.iter() {
                    segment.check(deps.api)?;
                    if !set.insert(segment.to_string()) {
                        return Err(StdError::generic_err(format!(
                            "Segment {} duplicated",
                            segment
                        )));
                    }
                }

                let start = &route[0];
                let end = &route[route.len() - 1];

                let config = RouteType::Path {
                    route: route.clone(),
                    router: Router(deps.api.addr_validate(&router)?),
                    router_type: router_type.clone(),
                };

                self.checked_save_route(deps.storage, (start, end), &config)?;

                let reverse_config = RouteType::Path {
                    route: route.clone().into_iter().rev().collect(),
                    router: Router(deps.api.addr_validate(&router)?),
                    router_type: router_type.reverse(),
                };

                self.checked_save_route(deps.storage, (end, start), &reverse_config)?;
            },
            RouteInit::PairProxy {
                single_direction_from,
                pair_contract,
            } => {
                let pair_contract = deps.api.addr_validate(&pair_contract)?;
                let pair_info = Pair(pair_contract).query_pair_info(&deps.querier)?;

                if pair_info.asset_infos.len() != 2 {
                    return Err(StdError::generic_err(
                        "Currently only pairs with 2 assets supported",
                    ));
                }

                let asset1 = &pair_info.asset_infos[0];
                let asset2 = &pair_info.asset_infos[1];

                let config = RouteType::PairProxy {
                    pair_info: pair_info.clone(),
                };

                if let Some(single_direction_from) = single_direction_from {
                    let start: &AssetInfo;
                    let end: &AssetInfo;

                    if single_direction_from.equal(asset1) {
                        start = asset1;
                        end = asset2;
                    } else if single_direction_from.equal(asset2) {
                        start = asset2;
                        end = asset1;
                    } else {
                        return Err(StdError::generic_err(format!(
                            "Provided start asset {} not in pair.",
                            single_direction_from
                        )));
                    }

                    self.checked_save_route(deps.storage, (start, end), &config)?;
                } else {
                    let start = asset1;
                    let end = asset2;

                    self.checked_save_route(deps.storage, (start, end), &config)?;
                    self.checked_save_route(deps.storage, (end, start), &config)?;
                }
            },
        };
        Ok(())
    }

    fn checked_save_route(
        &self,
        storage: &mut dyn Storage,
        key: (&AssetInfo, &AssetInfo),
        route_type: &RouteType,
    ) -> StdResult<()> {
        let binary_key = (key.0.as_bytes(), key.1.as_bytes());
        if self.routes.has(storage, binary_key) {
            return Err(StdError::generic_err("Route already registered"));
        }

        self.routes.save(
            storage,
            binary_key,
            &RouteConfig {
                key: (key.0.clone(), key.1.clone()),
                route_type: route_type.clone(),
            },
        )?;
        Ok(())
    }
}

pub fn validate_slippage(slippage_tolerance: Decimal) -> Result<Decimal, ContractError> {
    if slippage_tolerance > Decimal::percent(50) {
        Err(ContractError::SlippageToleranaceTooHigh)
    } else {
        Ok(slippage_tolerance)
    }
}

/// ## Description
/// Validates that commission bps must be less than or equal 10000
fn validate_commission(commission_bps: u64) -> StdResult<u64> {
    if commission_bps >= 10000u64 {
        Err(StdError::generic_err("commission rate must be 0 to 9999"))
    } else {
        Ok(commission_bps)
    }
}

/// ## Description
/// Validates that decimal value is in the range 0 to 1
fn validate_percentage(value: Decimal, field: &str) -> StdResult<Decimal> {
    if value > Decimal::one() {
        Err(StdError::generic_err(field.to_string() + " must be 0 to 1"))
    } else {
        Ok(value)
    }
}
