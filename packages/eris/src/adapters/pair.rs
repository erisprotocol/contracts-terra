use astroport::asset::{Asset, AssetInfo, AssetInfoExt};
use astroport::pair::{ConfigResponse, PoolResponse, QueryMsg, SimulationResponse};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, Decimal, QuerierWrapper, StdError, StdResult, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::compound_proxy::{PairInfo, PairInfoWw, PairType};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Pair(pub Addr);

#[cw_serde]
pub enum CustomExecuteMsg {
    /// Swap performs a swap in the pool
    Swap {
        offer_asset: Asset,
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<String>,
    },
    ProvideLiquidity {
        /// The assets available in the pool
        assets: Vec<Asset>,
        /// The slippage tolerance that allows liquidity provision only if the price in the pool doesn't move too much
        slippage_tolerance: Option<Decimal>,
        /// The receiver of LP tokens
        receiver: Option<String>,
    },
}

#[cw_serde]
pub enum CustomCw20HookMsg {
    /// Swap a given amount of asset
    Swap {
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<String>,
    },
}

impl Pair {
    pub fn query_pair_info(&self, querier: &QuerierWrapper) -> StdResult<PairInfo> {
        querier.query_wasm_smart(self.0.to_string(), &QueryMsg::Pair {})
    }
    pub fn query_ww_pair_info(&self, querier: &QuerierWrapper) -> StdResult<PairInfo> {
        let pair: PairInfoWw = querier.query_wasm_smart(self.0.to_string(), &QueryMsg::Pair {})?;
        Ok(PairInfo {
            asset_infos: pair.asset_infos.to_vec(),
            contract_addr: Addr::unchecked(pair.contract_addr),
            liquidity_token: match pair.liquidity_token {
                AssetInfo::Token {
                    contract_addr,
                } => contract_addr,
                AssetInfo::NativeToken {
                    ..
                } => return Err(StdError::generic_err("Only supports cw20 LP token pairs.")),
            },
            pair_type: match pair.pair_type {
                crate::compound_proxy::PairTypeWw::StableSwap {
                    ..
                } => PairType::StableWhiteWhale {},
                crate::compound_proxy::PairTypeWw::ConstantProduct => PairType::XykWhiteWhale {},
            },
        })
    }

    pub fn query_pool_info(&self, querier: &QuerierWrapper) -> StdResult<PoolResponse> {
        querier.query_wasm_smart(self.0.to_string(), &QueryMsg::Pool {})
    }

    pub fn query_config(&self, querier: &QuerierWrapper) -> StdResult<ConfigResponse> {
        querier.query_wasm_smart(self.0.to_string(), &QueryMsg::Config {})
    }

    pub fn simulate(
        &self,
        querier: &QuerierWrapper,
        offer_asset: &Asset,
        ask_asset_info: Option<AssetInfo>,
    ) -> StdResult<SimulationResponse> {
        querier.query_wasm_smart(
            self.0.to_string(),
            &QueryMsg::Simulation {
                offer_asset: offer_asset.clone(),
                ask_asset_info,
            },
        )
    }

    pub fn simulate_to_asset(
        &self,
        querier: &QuerierWrapper,
        pair_info: &PairInfo,
        offer_asset: &Asset,
    ) -> StdResult<Asset> {
        let ask_asset = if offer_asset.info == pair_info.asset_infos[0] {
            pair_info.asset_infos[1].clone()
        } else {
            pair_info.asset_infos[0].clone()
        };

        let simulation = self.simulate(querier, offer_asset, Some(ask_asset.clone()))?;

        Ok(ask_asset.with_balance(simulation.return_amount))
    }

    /// Generate msg for swapping specified asset
    pub fn swap_msg(
        &self,
        asset: &Asset,
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<String>,
    ) -> StdResult<CosmosMsg> {
        let wasm_msg = match &asset.info {
            AssetInfo::Token {
                contract_addr,
            } => WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: self.0.to_string(),
                    amount: asset.amount,
                    msg: to_binary(&CustomCw20HookMsg::Swap {
                        belief_price,
                        max_spread,
                        to,
                    })?,
                })?,
                funds: vec![],
            },

            AssetInfo::NativeToken {
                denom,
            } => WasmMsg::Execute {
                contract_addr: self.0.to_string(),
                msg: to_binary(&CustomExecuteMsg::Swap {
                    offer_asset: asset.clone(),
                    belief_price,
                    max_spread,
                    to,
                })?,
                funds: vec![Coin {
                    denom: denom.clone(),
                    amount: asset.amount,
                }],
            },
        };

        Ok(CosmosMsg::Wasm(wasm_msg))
    }

    pub fn provide_liquidity_msg(
        &self,
        assets: Vec<Asset>,
        slippage_tolerance: Option<Decimal>,
        receiver: Option<String>,
        mut funds: Vec<Coin>,
    ) -> StdResult<CosmosMsg> {
        funds.sort_by(|a, b| a.denom.cmp(&b.denom));
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            msg: to_binary(&CustomExecuteMsg::ProvideLiquidity {
                assets,
                slippage_tolerance,
                receiver,
            })?,
            funds,
        }))
    }
}
