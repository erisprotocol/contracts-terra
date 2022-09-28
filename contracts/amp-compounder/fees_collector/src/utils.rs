use crate::error::ContractError;
use crate::state::{Config, BRIDGES};
use astroport::asset::{Asset, AssetInfo, PairInfo};
use astroport::querier::query_pair_info;
use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, Deps, Env, QuerierWrapper, StdResult, Uint128, WasmMsg,
};
use eris::adapters::pair::Pair;
use eris::fees_collector::ExecuteMsg;

/// The default bridge depth for a fee token
pub const BRIDGES_INITIAL_DEPTH: u64 = 0;
/// Maximum amount of bridges to use in a multi-hop swap
pub const BRIDGES_MAX_DEPTH: u64 = 2;
/// Swap execution depth limit
pub const BRIDGES_EXECUTION_MAX_DEPTH: u64 = 3;

/// Creates swap message
pub fn try_build_swap_msg(
    querier: &QuerierWrapper,
    config: &Config,
    from: AssetInfo,
    to: AssetInfo,
    amount: Uint128,
) -> Result<CosmosMsg, ContractError> {
    let pool = query_pair_info(querier, config.factory_contract.clone(), &[from.clone(), to])?;
    let msg = Pair(pool.contract_addr).swap_msg(
        &Asset {
            info: from,
            amount,
        },
        None,
        Some(config.max_spread),
        None,
    )?;
    Ok(msg)
}

/// Creates swap message via bridge token pair
pub fn build_swap_bridge_msg(
    env: Env,
    bridge_assets: Vec<AssetInfo>,
    depth: u64,
) -> StdResult<CosmosMsg> {
    let msg: CosmosMsg =
        // Swap bridge assets
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&ExecuteMsg::SwapBridgeAssets {
                assets: bridge_assets,
                depth,
            })?,
            funds: vec![],
        });

    Ok(msg)
}

/// Validates bridge token
pub fn validate_bridge(
    deps: Deps,
    factory_contract: Addr,
    from_token: AssetInfo,
    bridge_token: AssetInfo,
    stablecoin_token: AssetInfo,
    depth: u64,
) -> Result<PairInfo, ContractError> {
    // Check if the bridge pool exists
    let bridge_pool = query_pair_info(
        &deps.querier,
        factory_contract.clone(),
        &[from_token.clone(), bridge_token.clone()],
    )?;

    // Check if the bridge token - stablecoin pool exists
    let stablecoin_pool = query_pair_info(
        &deps.querier,
        factory_contract.clone(),
        &[bridge_token.clone(), stablecoin_token.clone()],
    );
    if stablecoin_pool.is_err() {
        if depth >= BRIDGES_MAX_DEPTH {
            return Err(ContractError::MaxBridgeDepth(depth));
        }

        // Check if next level of bridge exists
        let next_bridge_token = BRIDGES
            .load(deps.storage, bridge_token.to_string())
            .map_err(|_| ContractError::InvalidBridgeDestination(from_token.clone()))?;

        validate_bridge(
            deps,
            factory_contract,
            bridge_token,
            next_bridge_token,
            stablecoin_token,
            depth + 1,
        )?;
    }

    Ok(bridge_pool)
}
