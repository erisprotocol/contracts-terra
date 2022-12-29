use cosmwasm_std::{Binary, CosmosMsg, StdError, StdResult};
use protobuf::{well_known_types::any::Any, Message};

use self::{
    authz::MsgExec,
    proto::{MsgExecuteContract, MsgWithdrawDelegatorReward},
};

pub mod authz;
pub mod msgex;
pub mod proto;

impl MsgExec {
    pub fn to_cosmos_msg(&self) -> CosmosMsg {
        let exec_bytes: Vec<u8> = self.write_to_bytes().unwrap();

        CosmosMsg::Stargate {
            type_url: "/cosmos.authz.v1beta1.MsgExec".to_string(),
            value: Binary::from(exec_bytes),
        }
    }
}

impl MsgWithdrawDelegatorReward {
    pub(crate) fn to_any(&self) -> StdResult<Any> {
        self.write_to_bytes()
            .map(|bytes| Any {
                type_url: "/cosmos.distribution.v1beta1.MsgWithdrawDelegatorReward".to_string(),
                value: bytes,
                special_fields: Default::default(),
            })
            .map_err(|e| StdError::generic_err(e.to_string()))
    }
}

impl MsgExecuteContract {
    pub(crate) fn to_any(&self) -> StdResult<Any> {
        self.write_to_bytes()
            .map(|bytes| Any {
                type_url: "/cosmwasm.wasm.v1.MsgExecuteContract".to_string(),
                value: bytes,
                special_fields: Default::default(),
            })
            .map_err(|e| StdError::generic_err(e.to_string()))
    }
}
