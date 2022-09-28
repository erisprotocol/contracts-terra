use astroport::asset::{Asset, AssetInfo};
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Coin, CosmosMsg, MessageInfo, StdError, StdResult, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Expiration};

use crate::fees_collector::TargetConfigChecked;

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
}
