use astroport::asset::AssetInfo;
use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;
use eris::{
    adapters::{factory::Factory, pair::Pair},
    compound_proxy::{
        ConfigResponse, LpConfig, LpStateResponse, RouteResponseItem, SupportsSwapResponse,
    },
};

use crate::{
    constants::{DEFAULT_LIMIT, MAX_LIMIT},
    state::State,
};

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = State::default();
    state.config.load(deps.storage).map(|c| ConfigResponse {
        factory: c.factory.map(|f| f.0),
        owner: c.owner,
    })
}

pub fn get_lp(deps: Deps, lp_addr: String) -> StdResult<LpConfig> {
    let state = State::default();
    state.lps.load(deps.storage, lp_addr)
}

pub fn get_lp_state(deps: Deps, lp_addr: String) -> StdResult<LpStateResponse> {
    let state = State::default();
    let lp = state.lps.load(deps.storage, lp_addr)?;

    let pair = Pair(lp.pair_info.contract_addr.clone());
    let pool_response = pair.query_pool_info(&deps.querier)?;

    Ok(LpStateResponse {
        contract_addr: lp.pair_info.contract_addr,
        liquidity_token: lp.pair_info.liquidity_token,
        total_share: pool_response.total_share,
        assets: pool_response.assets,
    })
}

pub fn get_lps(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<LpConfig>> {
    let state = State::default();

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    state
        .lps
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, v) = item?;
            Ok(v)
        })
        .collect()
}

pub fn get_routes(
    deps: Deps,
    start_after: Option<(AssetInfo, AssetInfo)>,
    limit: Option<u32>,
) -> StdResult<Vec<RouteResponseItem>> {
    let state = State::default();

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let owned: (AssetInfo, AssetInfo);
    let start = if let Some(start_after) = start_after {
        owned = start_after;
        Some(Bound::exclusive((owned.0.as_bytes(), owned.1.as_bytes())))
    } else {
        None
    };

    state
        .routes
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, v) = item?;

            Ok(RouteResponseItem {
                key: (v.key.0.to_string(), v.key.1.to_string()),
                route_type: match v.route_type {
                    crate::state::RouteType::Path {
                        router,
                        router_type,
                        route,
                    } => eris::compound_proxy::RouteTypeResponseItem::Path {
                        router: router.0.to_string(),
                        router_type,
                        route: route.into_iter().map(|s| s.to_string()).collect(),
                    },
                    crate::state::RouteType::PairProxy {
                        pair_info,
                    } => eris::compound_proxy::RouteTypeResponseItem::PairProxy {
                        pair_contract: pair_info.contract_addr.to_string(),
                        asset_infos: pair_info
                            .asset_infos
                            .into_iter()
                            .map(|s| s.to_string())
                            .collect(),
                    },
                },
            })
        })
        .collect()
}

pub fn get_route(deps: Deps, from: AssetInfo, to: AssetInfo) -> StdResult<RouteResponseItem> {
    let state = State::default();

    let key: (&[u8], &[u8]) = (from.as_bytes(), to.as_bytes());
    let route_config = state.routes.load(deps.storage, key)?;

    Ok(RouteResponseItem {
        key: (route_config.key.0.to_string(), route_config.key.1.to_string()),
        route_type: match route_config.route_type {
            crate::state::RouteType::Path {
                router,
                router_type,
                route,
            } => eris::compound_proxy::RouteTypeResponseItem::Path {
                router: router.0.to_string(),
                router_type,
                route: route.into_iter().map(|s| s.to_string()).collect(),
            },
            crate::state::RouteType::PairProxy {
                pair_info,
            } => eris::compound_proxy::RouteTypeResponseItem::PairProxy {
                pair_contract: pair_info.contract_addr.to_string(),
                asset_infos: pair_info.asset_infos.into_iter().map(|s| s.to_string()).collect(),
            },
        },
    })
}

pub fn query_supports_swap(
    deps: Deps,
    from: AssetInfo,
    to: AssetInfo,
) -> StdResult<SupportsSwapResponse> {
    let suppored = if from == to {
        true
    } else {
        let state = State::default();
        let key = (from.as_bytes(), to.as_bytes());
        let route_config = state.routes.load(deps.storage, key);

        if route_config.is_ok() {
            true
        } else {
            let factory: Option<Factory> = state.config.load(deps.storage)?.factory;
            if let Some(factory) = &factory {
                factory.get_pair(&deps.querier, from, to).is_ok()
            } else {
                false
            }
        }
    };

    Ok(SupportsSwapResponse {
        suppored,
    })
}
