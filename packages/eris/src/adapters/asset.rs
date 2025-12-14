use std::collections::HashMap;

use astroport::asset::{Asset, AssetInfo};
use cosmwasm_std::{
    coin, to_json_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Env, IbcTimeout, MessageInfo,
    QuerierWrapper, StdError, StdResult, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Expiration};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::adapters::msgs_zapper::AssetInfoUnchecked;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Ics20TransferMsg {
    pub channel: String,
    pub remote_address: String,
}

pub trait AssetInfoEx {
    fn to_new(&self) -> AssetInfoUnchecked;
}

impl AssetInfoEx for AssetInfo {
    fn to_new(&self) -> AssetInfoUnchecked {
        match self {
            AssetInfo::Token {
                contract_addr,
            } => AssetInfoUnchecked::Cw20(contract_addr.to_string()),
            AssetInfo::NativeToken {
                denom,
            } => AssetInfoUnchecked::Native(denom.clone()),
        }
    }
}

pub trait AssetInfosEx {
    fn query_balances(&self, querier: &QuerierWrapper, address: &Addr) -> StdResult<Vec<Asset>>;
    fn to_new(&self) -> Vec<AssetInfoUnchecked>;
}

impl AssetInfosEx for Vec<AssetInfo> {
    fn query_balances(&self, querier: &QuerierWrapper, address: &Addr) -> StdResult<Vec<Asset>> {
        let assets: Vec<Asset> = self
            .iter()
            .map(|asset| {
                let result = asset.query_pool(querier, address)?;
                Ok(Asset {
                    info: asset.clone(),
                    amount: result,
                })
            })
            .collect::<StdResult<_>>()?;

        Ok(assets.into_iter().collect())
    }

    fn to_new(&self) -> Vec<AssetInfoUnchecked> {
        self.iter().map(|asset| asset.to_new()).collect()
    }
}

pub trait AssetsEx {
    fn query_balance_diff(
        self,
        querier: &QuerierWrapper,
        address: &Addr,
        max_amount: Option<Vec<Asset>>,
    ) -> StdResult<Vec<Asset>>;
}

impl AssetsEx for Vec<Asset> {
    fn query_balance_diff(
        self,
        querier: &QuerierWrapper,
        address: &Addr,
        max_amount: Option<Vec<Asset>>,
    ) -> StdResult<Vec<Asset>> {
        let hash_map = max_amount.map(|max| {
            let hash: HashMap<AssetInfo, Uint128> =
                max.into_iter().map(|asset| (asset.info, asset.amount)).collect();
            hash
        });

        let assets: Vec<Asset> = self
            .into_iter()
            .map(|asset| {
                let result = asset.info.query_pool(querier, address)?;
                let mut amount = result.checked_sub(asset.amount)?;

                if let Some(hash_map) = &hash_map {
                    if let Some(max) = hash_map.get(&asset.info) {
                        if !max.is_zero() {
                            amount = std::cmp::min(amount, *max);
                        }
                    }
                }

                Ok(Asset {
                    info: asset.info,
                    amount,
                })
            })
            .collect::<StdResult<_>>()?;

        Ok(assets.into_iter().filter(|asset| !asset.amount.is_zero()).collect())
    }
}

pub trait AssetEx {
    fn transfer_msg(&self, to: &Addr) -> StdResult<CosmosMsg>;
    fn transfer_msg_target(&self, to_addr: &Addr, to_msg: Option<Binary>) -> StdResult<CosmosMsg>;
    fn transfer_from_msg(&self, from: &Addr, to: &Addr) -> StdResult<CosmosMsg>;
    fn increase_allowance_msg(
        &self,
        spender: String,
        expires: Option<Expiration>,
    ) -> StdResult<CosmosMsg>;

    fn deposit_asset(
        &self,
        info: &MessageInfo,
        recipient: &Addr,
        messages: &mut Vec<CosmosMsg>,
    ) -> StdResult<()>;

    fn transfer_msg_ibc(
        &self,
        env: &Env,
        to_addr: String,
        channel_id: String,
        ics20: Option<String>,
    ) -> StdResult<CosmosMsg>;

    fn send_or_execute_msg<A: Into<String>, T: Serialize + Sized>(
        &self,
        contract: A,
        msg: &T,
    ) -> Result<CosmosMsg, StdError>;

    fn send_or_execute_msg_binary<A: Into<String>>(
        &self,
        contract: A,
        msg: Binary,
    ) -> Result<CosmosMsg, StdError>;

    fn to_coin(&self) -> StdResult<Coin>;
}

impl AssetEx for Asset {
    fn transfer_msg(&self, to: &Addr) -> StdResult<CosmosMsg> {
        match &self.info {
            AssetInfo::Token {
                contract_addr,
            } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: to.to_string(),
                    amount: self.amount,
                })?,
                funds: vec![],
            })),
            AssetInfo::NativeToken {
                denom,
            } => Ok(CosmosMsg::Bank(BankMsg::Send {
                to_address: to.to_string(),
                amount: vec![Coin {
                    denom: denom.to_string(),
                    amount: self.amount,
                }],
            })),
        }
    }

    fn transfer_msg_target(&self, to_addr: &Addr, to_msg: Option<Binary>) -> StdResult<CosmosMsg> {
        if let Some(msg) = to_msg {
            match &self.info {
                AssetInfo::Token {
                    contract_addr,
                } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Send {
                        contract: to_addr.to_string(),
                        amount: self.amount,
                        msg,
                    })?,
                    funds: vec![],
                })),
                AssetInfo::NativeToken {
                    denom,
                } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: to_addr.to_string(),
                    msg,
                    funds: vec![Coin {
                        denom: denom.to_string(),
                        amount: self.amount,
                    }],
                })),
            }
        } else {
            self.transfer_msg(to_addr)
        }
    }

    fn transfer_msg_ibc(
        &self,
        env: &Env,
        to_addr: String,
        channel: String,
        ics20: Option<String>,
    ) -> StdResult<CosmosMsg> {
        match &self.info {
            AssetInfo::Token {
                contract_addr,
            } => {
                if let Some(ics20) = ics20 {
                    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: contract_addr.to_string(),
                        msg: to_json_binary(&Cw20ExecuteMsg::Send {
                            contract: ics20,
                            amount: self.amount,
                            msg: to_json_binary(&Ics20TransferMsg {
                                channel,
                                remote_address: to_addr,
                            })?,
                        })?,
                        funds: vec![],
                    }))
                } else {
                    Err(StdError::generic_err("ICS20 not setup correctly"))
                }
            },

            AssetInfo::NativeToken {
                denom,
            } => Ok(CosmosMsg::Ibc(cosmwasm_std::IbcMsg::Transfer {
                channel_id: channel,
                to_address: to_addr,
                amount: coin(self.amount.u128(), denom),
                timeout: IbcTimeout::with_timestamp(env.block.time.plus_seconds(10 * 60)),
            })),
        }
    }

    fn transfer_from_msg(&self, from: &Addr, to: &Addr) -> StdResult<CosmosMsg> {
        match &self.info {
            AssetInfo::Token {
                contract_addr,
            } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: from.to_string(),
                    recipient: to.to_string(),
                    amount: self.amount,
                })?,
                funds: vec![],
            })),
            AssetInfo::NativeToken {
                ..
            } => Err(StdError::generic_err("TransferFrom does not apply to native tokens")),
        }
    }

    fn increase_allowance_msg(
        &self,
        spender: String,
        expires: Option<Expiration>,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.info.to_string(),
            msg: to_json_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                spender,
                amount: self.amount,
                expires,
            })?,
            funds: vec![],
        }))
    }

    fn deposit_asset(
        &self,
        info: &MessageInfo,
        recipient: &Addr,
        messages: &mut Vec<CosmosMsg>,
    ) -> StdResult<()> {
        if self.amount.is_zero() {
            return Ok(());
        }

        match &self.info {
            AssetInfo::Token {
                ..
            } => {
                messages.push(self.transfer_from_msg(&info.sender, recipient)?);
            },
            AssetInfo::NativeToken {
                ..
            } => {
                self.assert_sent_native_token_balance(info)?;
            },
        };
        Ok(())
    }

    fn send_or_execute_msg<A: Into<String>, T: Serialize + Sized>(
        &self,
        contract: A,
        msg: &T,
    ) -> Result<CosmosMsg, StdError> {
        self.send_or_execute_msg_binary(contract, to_json_binary(msg)?)
    }

    fn send_or_execute_msg_binary<A: Into<String>>(
        &self,
        contract: A,
        msg: Binary,
    ) -> Result<CosmosMsg, StdError> {
        match &self.info {
            AssetInfo::Token {
                contract_addr,
            } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.into(),
                msg: to_json_binary(&Cw20ExecuteMsg::Send {
                    contract: contract.into(),
                    amount: self.amount,
                    msg,
                })?,
                funds: vec![],
            })),
            AssetInfo::NativeToken {
                denom,
            } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract.into(),
                msg,
                funds: vec![coin(self.amount.u128(), denom)],
            })),
        }
    }

    fn to_coin(&self) -> StdResult<Coin> {
        match &self.info {
            AssetInfo::Token {
                ..
            } => Err(StdError::generic_err("Cannot convert CW20 token to Coin")),
            AssetInfo::NativeToken {
                denom,
            } => Ok(Coin {
                denom: denom.to_string(),
                amount: self.amount,
            }),
        }
    }
}
