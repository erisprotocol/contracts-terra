use cosmwasm_std::{CosmosMsg, Env, StdError, StdResult};
use itertools::Itertools;
use protobuf::SpecialFields;

use super::{
    authz::MsgExec,
    proto::{Coin, MsgExecuteContract},
};

pub trait CosmosMsgEx {
    fn to_proto_msg(self, sender: impl Into<String>) -> StdResult<MsgExecuteContract>;
    fn to_authz_msg(self, sender: impl Into<String>, env: &Env) -> StdResult<CosmosMsg>;
}

impl CosmosMsgEx for CosmosMsg {
    fn to_proto_msg(self, sender: impl Into<String>) -> StdResult<MsgExecuteContract> {
        match self {
            CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                contract_addr,
                msg,
                funds,
            }) => Ok(MsgExecuteContract {
                sender: sender.into(),
                contract: contract_addr,
                msg: msg.to_vec(),
                funds: funds
                    .into_iter()
                    .map(|c| Coin {
                        amount: c.amount.to_string(),
                        denom: c.denom,
                        special_fields: SpecialFields::default(),
                    })
                    .collect_vec(),
                special_fields: SpecialFields::default(),
            }),
            _ => Err(StdError::generic_err("can not convert to proto msg")),
        }
    }

    fn to_authz_msg(self, sender: impl Into<String>, env: &Env) -> StdResult<CosmosMsg> {
        self.to_proto_msg(sender)?.to_cosmos_msg(env)
    }
}

pub trait MsgExecuteContractEx {
    fn to_cosmos_msg(self, env: &Env) -> StdResult<CosmosMsg>;
}

impl MsgExecuteContractEx for MsgExecuteContract {
    fn to_cosmos_msg(self, env: &Env) -> StdResult<CosmosMsg> {
        let mut exec = MsgExec::new();
        exec.grantee = env.contract.address.to_string();
        exec.msgs = vec![self.to_any()?];
        Ok(exec.to_cosmos_msg())
    }
}
