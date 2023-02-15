use astroport::asset::{Asset, AssetInfo};
use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, Decimal, QuerierWrapper, StdError, StdResult, Uint128,
    WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RouterType {
    AstroSwap,
    TerraSwap,
    TokenSwap,
}

impl RouterType {
    pub fn create_swap_operations(
        &self,
        asset_infos: &[AssetInfo],
    ) -> StdResult<Vec<SwapOperation>> {
        if let Some((first, tails)) = asset_infos.split_first() {
            let mut swap_operations: Vec<SwapOperation> = vec![];
            let mut previous = first.clone();
            for asset_info in tails {
                let offer_asset_info = previous;
                let ask_asset_info = asset_info.clone();
                let op = match self {
                    RouterType::AstroSwap => SwapOperation::AstroSwap {
                        offer_asset_info,
                        ask_asset_info,
                    },
                    RouterType::TerraSwap => SwapOperation::TerraSwap {
                        offer_asset_info,
                        ask_asset_info,
                    },
                    RouterType::TokenSwap => SwapOperation::TokenSwap {
                        offer_asset_info,
                        ask_asset_info,
                    },
                };
                swap_operations.push(op);
                previous = asset_info.clone();
            }
            Ok(swap_operations)
        } else {
            Err(StdError::generic_err("required asset"))
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SwapOperation {
    AstroSwap {
        offer_asset_info: AssetInfo,
        ask_asset_info: AssetInfo,
    },
    TerraSwap {
        offer_asset_info: AssetInfo,
        ask_asset_info: AssetInfo,
    },
    TokenSwap {
        offer_asset_info: AssetInfo,
        ask_asset_info: AssetInfo,
    },
}

impl SwapOperation {
    pub fn get_offer_asset_info(&self) -> AssetInfo {
        match self {
            SwapOperation::AstroSwap {
                offer_asset_info,
                ..
            } => offer_asset_info.clone(),
            SwapOperation::TerraSwap {
                offer_asset_info,
                ..
            } => offer_asset_info.clone(),
            SwapOperation::TokenSwap {
                offer_asset_info,
                ..
            } => offer_asset_info.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ExecuteSwapOperations {
        operations: Vec<SwapOperation>,
        minimum_receive: Option<Uint128>,
        to: Option<Addr>,
        max_spread: Option<Decimal>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    ExecuteSwapOperations {
        operations: Vec<SwapOperation>,
        minimum_receive: Option<Uint128>,
        to: Option<Addr>,
        max_spread: Option<Decimal>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    SimulateSwapOperations {
        offer_amount: Uint128,
        operations: Vec<SwapOperation>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct SimulateSwapOperationsResponse {
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Router(pub Addr);

impl Router {
    pub fn simulate(
        &self,
        querier: &QuerierWrapper,
        offer_amount: Uint128,
        operations: Vec<SwapOperation>,
    ) -> StdResult<SimulateSwapOperationsResponse> {
        querier.query_wasm_smart(
            self.0.to_string(),
            &QueryMsg::SimulateSwapOperations {
                offer_amount,
                operations,
            },
        )
    }

    pub fn execute_swap_operations_msg(
        &self,
        offer_asset: Asset,
        operations: Vec<SwapOperation>,
        minimum_receive: Option<Uint128>,
        to: Option<Addr>,
        max_spread: Option<Decimal>,
    ) -> StdResult<CosmosMsg> {
        let wasm_msg = match &offer_asset.info {
            AssetInfo::Token {
                contract_addr,
            } => WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: self.0.to_string(),
                    amount: offer_asset.amount,
                    msg: to_binary(&Cw20HookMsg::ExecuteSwapOperations {
                        operations,
                        minimum_receive,
                        to,
                        max_spread,
                    })?,
                })?,
                funds: vec![],
            },
            AssetInfo::NativeToken {
                denom,
            } => WasmMsg::Execute {
                contract_addr: self.0.to_string(),
                msg: to_binary(&ExecuteMsg::ExecuteSwapOperations {
                    operations,
                    minimum_receive,
                    to,
                    max_spread,
                })?,
                funds: vec![Coin {
                    denom: denom.clone(),
                    amount: offer_asset.amount,
                }],
            },
        };

        Ok(CosmosMsg::Wasm(wasm_msg))
    }
}
