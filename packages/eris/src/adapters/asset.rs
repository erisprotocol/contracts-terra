use std::collections::HashMap;

use astroport::asset::{Asset, AssetInfo, AssetInfoExt};
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Coin, CosmosMsg, MessageInfo, QuerierWrapper, StdError, StdResult,
    Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Expiration};

use crate::{fees_collector::TargetConfigChecked, helpers::bps::BasicPoints};

pub trait AssetInfosEx {
    fn query_balances(&self, querier: &QuerierWrapper, address: &Addr) -> StdResult<Vec<Asset>>;
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

        Ok(assets.into_iter().filter(|asset| !asset.amount.is_zero()).collect())
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

// impl fmt::Display for Vec<Asset> {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

//         let mut comma_separated = String::new();

//         for num in &self.0[0..self.0.len() - 1] {
//             comma_separated.push_str(&num.to_string());
//             comma_separated.push_str(", ");
//         }

//         comma_separated.push_str(&self.0[self.0.len() - 1].to_string());
//         write!(f, "{}", comma_separated)
//     }
// }

// impl Display for Vec<Asset> {
//     fn fmt(&self, f: &mut Formatter) -> Result {
//         Coin
//         let mut comma_separated = String::new();

//         for num in &self.0[0..self.0.len() - 1] {
//             comma_separated.push_str(&num.to_string());
//             comma_separated.push_str(", ");
//         }

//         comma_separated.push_str(&self.0[self.0.len() - 1].to_string());
//         write!(f, "{}", comma_separated)
//     }
// }

pub trait AssetEx {
    fn transfer_msg(&self, to: &Addr) -> StdResult<CosmosMsg>;
    fn transfer_msg_target(&self, to: &TargetConfigChecked) -> StdResult<CosmosMsg>;
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

    fn subtract_fee(&self, fee: BasicPoints) -> (Asset, Uint128);
}

impl AssetEx for Asset {
    fn transfer_msg(&self, to: &Addr) -> StdResult<CosmosMsg> {
        match &self.info {
            AssetInfo::Token {
                contract_addr,
            } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
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

    fn transfer_msg_target(&self, to: &TargetConfigChecked) -> StdResult<CosmosMsg> {
        if let Some(msg) = to.msg.clone() {
            match &self.info {
                AssetInfo::Token {
                    contract_addr,
                } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Send {
                        contract: to.addr.to_string(),
                        amount: self.amount,
                        msg,
                    })?,
                    funds: vec![],
                })),
                AssetInfo::NativeToken {
                    denom,
                } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: to.addr.to_string(),
                    msg,
                    funds: vec![Coin {
                        denom: denom.to_string(),
                        amount: self.amount,
                    }],
                })),
            }
        } else {
            self.transfer_msg(&to.addr)
        }
    }

    fn transfer_from_msg(&self, from: &Addr, to: &Addr) -> StdResult<CosmosMsg> {
        match &self.info {
            AssetInfo::Token {
                contract_addr,
            } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
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
            msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
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

    fn subtract_fee(&self, fee_percent: BasicPoints) -> (Asset, Uint128) {
        let fee_absolute = self.amount * fee_percent.decimal();
        let new_amount = self.amount.saturating_sub(fee_absolute);
        (self.info.with_balance(new_amount), fee_absolute)
    }
}
