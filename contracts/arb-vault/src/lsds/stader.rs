use astroport::asset::{token_asset_info, AssetInfo};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, Decimal, Deps, QueryRequest, Timestamp, Uint128, WasmMsg, WasmQuery,
};
use cw20::Cw20ExecuteMsg;
use stader::{
    msg::{Cw20HookMsg, ExecuteMsg, QueryMsg, QueryStateResponse},
    state::UndelegationInfo,
};

use crate::error::{adapter_error, adapter_error_empty, CustomResult};

use super::lsdadapter::LsdAdapter;

// needed to be done manually, as they are private in the package
#[cw_serde]
pub struct QueryBatchUndelegationResponse {
    pub batch: Option<BatchUndelegationRecord>,
}

#[cw_serde]
pub struct BatchUndelegationRecord {
    pub(crate) undelegated_tokens: Uint128,
    pub(crate) create_time: Timestamp,
    pub(crate) est_release_time: Option<Timestamp>,
    pub(crate) reconciled: bool,
    pub(crate) undelegation_er: Decimal,
    pub(crate) undelegated_stake: Uint128,
    pub(crate) unbonding_slashing_ratio: Decimal, // If Unbonding slashing happens during the 21 day period.
}

pub type GetUserUndelegationRecordsResponse = Vec<UndelegationInfo>;

pub struct Stader {
    pub undelegation_records_cache: Option<Vec<UndelegationCacheItem>>,
    pub state_cache: Option<QueryStateResponse>,

    pub wallet: Addr,
    pub addr: Addr,
    pub cw20: Addr,
}

impl Stader {
    fn query_state(
        &mut self,
        deps: &Deps,
    ) -> Result<QueryStateResponse, crate::error::ContractError> {
        deps.querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: self.addr.to_string(),
                msg: to_binary(&QueryMsg::State {}).unwrap(),
            }))
            .map_err(|a| adapter_error("stader", "query_state", a))
    }

    fn query_user_undelegation_records(
        &mut self,
        deps: &Deps,
    ) -> Result<Vec<UndelegationInfo>, crate::error::ContractError> {
        deps.querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: self.addr.to_string(),
                msg: to_binary(&QueryMsg::GetUserUndelegationRecords {
                    user_addr: self.wallet.to_string(),
                    limit: Some(20),
                    start_after: None,
                })
                .unwrap(),
            }))
            .map_err(|a| adapter_error("stader", "query_user_undelegation_records", a))
    }

    fn query_batch_undelegation(
        &mut self,
        deps: &Deps,
        batch_id: u64,
    ) -> CustomResult<QueryBatchUndelegationResponse> {
        let batch_info: QueryBatchUndelegationResponse = deps
            .querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: self.addr.to_string(),
                msg: to_binary(&QueryMsg::BatchUndelegation {
                    batch_id,
                })
                .unwrap(),
            }))
            .map_err(|a| adapter_error("stader", "query_batch_undelegation", a))?;
        Ok(batch_info)
    }

    fn get_unbond_msg(&self, amount: Uint128) -> CustomResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.cw20.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: self.addr.to_string(),
                amount,
                msg: to_binary(&Cw20HookMsg::QueueUndelegate {})?,
            })?,
        }))
    }

    fn _get_withdraw_funds_msg(&self, batch_id: u64) -> CustomResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.addr.to_string(),
            funds: vec![],
            msg: to_binary(&ExecuteMsg::WithdrawFundsToWallet {
                batch_id,
            })?,
        }))
    }

    fn cached_undelegation_records(
        &mut self,
        deps: &Deps,
    ) -> CustomResult<&Vec<UndelegationCacheItem>> {
        if self.undelegation_records_cache.is_none() {
            let undelegation_records: GetUserUndelegationRecordsResponse =
                self.query_user_undelegation_records(deps)?;

            let result = undelegation_records
                .iter()
                .map(|a| {
                    let batch_info = self.query_batch_undelegation(deps, a.batch_id)?;

                    let batch = batch_info
                        .batch
                        .ok_or_else(|| adapter_error_empty("stader", "expected batch"))?;

                    Ok(UndelegationCacheItem {
                        token_amount: a.token_amount,
                        reconciled: batch.reconciled,
                        exchange_rate: batch.undelegation_er,
                        batch_id: a.batch_id,
                    })
                })
                .collect::<CustomResult<Vec<UndelegationCacheItem>>>()?;

            self.undelegation_records_cache = Some(result);
        }

        Ok(self.undelegation_records_cache.as_ref().unwrap())
    }

    fn cached_state(&mut self, deps: &Deps) -> CustomResult<&QueryStateResponse> {
        if self.state_cache.is_none() {
            let result = self.query_state(deps)?;
            self.state_cache = Some(result);
        }

        Ok(self.state_cache.as_ref().unwrap())
    }
}

pub struct UndelegationCacheItem {
    pub token_amount: Uint128,
    pub reconciled: bool,
    pub exchange_rate: Decimal,
    pub batch_id: u64,
}

impl LsdAdapter for Stader {
    fn used_contracts(&self) -> Vec<Addr> {
        vec![self.cw20.clone(), self.addr.clone()]
    }

    fn asset(&self) -> AssetInfo {
        token_asset_info(self.cw20.clone())
    }

    fn unbond(&self, _deps: &Deps, amount: Uint128) -> CustomResult<Vec<CosmosMsg>> {
        Ok(vec![self.get_unbond_msg(amount)?])
    }

    fn query_unbonding(&mut self, deps: &Deps) -> CustomResult<Uint128> {
        Ok(self
            .cached_undelegation_records(deps)?
            .iter()
            .filter(|a| !a.reconciled)
            .map(|batch| batch.exchange_rate * batch.token_amount)
            .sum())
    }

    fn withdraw(&mut self, deps: &Deps, _amount: Uint128) -> CustomResult<Vec<CosmosMsg>> {
        let contract = self.addr.to_string();
        let msgs = self
            .cached_undelegation_records(deps)?
            .iter()
            .filter(|a| a.reconciled)
            .map(|a| {
                // would like to call get_withdraw_funds_msg
                // but it has issues with mutable borrows
                Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract.clone(),
                    funds: vec![],
                    msg: to_binary(&ExecuteMsg::WithdrawFundsToWallet {
                        batch_id: a.batch_id,
                    })?,
                }))
            })
            .collect::<CustomResult<Vec<CosmosMsg>>>()?;
        Ok(msgs)
    }

    fn query_withdrawable(&mut self, deps: &Deps) -> CustomResult<Uint128> {
        Ok(self
            .cached_undelegation_records(deps)?
            .iter()
            .filter(|a| a.reconciled)
            .map(|batch| batch.exchange_rate * batch.token_amount)
            .sum())
    }

    fn query_factor_x_to_normal(&mut self, deps: &Deps) -> CustomResult<Decimal> {
        Ok(self.cached_state(deps)?.state.exchange_rate)
    }
}
