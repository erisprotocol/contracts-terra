use astroport::asset::{token_asset_info, AssetInfo};
use cw20::Cw20ExecuteMsg;

use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, Decimal, Deps, QueryRequest, Uint128, WasmMsg, WasmQuery,
};
use prism::hub::{
    AllHistoryResponse, Cw20HookMsg, ExecuteMsg, QueryMsg, StateResponse, UnbondRequestsResponse,
    WithdrawableUnbondedResponse,
};

use crate::error::{adapter_error, adapter_error_empty, CustomResult};

use super::lsdadapter::LsdAdapter;

pub struct Prism {
    pub unbonding_cache: Option<Uint128>,
    pub state_cache: Option<StateResponse>,

    pub wallet: Addr,
    pub addr: Addr,
    pub cw20: Addr,
}

impl Prism {
    fn query_single_history(&mut self, deps: &Deps, id: u64) -> CustomResult<AllHistoryResponse> {
        deps.querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: self.addr.to_string(),
                msg: to_binary(&QueryMsg::AllHistory {
                    limit: Some(1u32),
                    start_from: id.checked_sub(1).or(None),
                })
                .unwrap(),
            }))
            .map_err(|a| adapter_error("prism", "query_all_history", a))
    }

    fn query_unbond_requests(&mut self, deps: &Deps) -> CustomResult<UnbondRequestsResponse> {
        deps.querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: self.addr.to_string(),
                msg: to_binary(&QueryMsg::UnbondRequests {
                    address: self.wallet.to_string(),
                })
                .unwrap(),
            }))
            .map_err(|a| adapter_error("prism", "query_unbond_requests", a))
    }

    fn query_withdrawable_unbonded(
        &mut self,
        deps: &Deps,
    ) -> CustomResult<WithdrawableUnbondedResponse> {
        deps.querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: self.addr.to_string(),
                msg: to_binary(&QueryMsg::WithdrawableUnbonded {
                    address: self.wallet.to_string(),
                })
                .unwrap(),
            }))
            .map_err(|a| adapter_error("prism", "query_withdrawable_unbonded", a))
    }

    fn query_state(&mut self, deps: &Deps) -> CustomResult<StateResponse> {
        let result: StateResponse = deps
            .querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: self.addr.to_string(),
                msg: to_binary(&QueryMsg::State {}).unwrap(),
            }))
            .map_err(|a| adapter_error("prism", "query_state", a))?;
        Ok(result)
    }

    fn get_unbond_msg(&self, amount: Uint128) -> CustomResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.cw20.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: self.addr.to_string(),
                amount,
                msg: to_binary(&Cw20HookMsg::Unbond {})?,
            })?,
        }))
    }

    fn get_withdraw_msg(&self) -> CustomResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.addr.to_string(),
            funds: vec![],
            msg: to_binary(&ExecuteMsg::WithdrawUnbonded {})?,
        }))
    }

    fn cached_unbonding(&mut self, deps: &Deps) -> CustomResult<Uint128> {
        if self.unbonding_cache.is_none() {
            // get all current unbond requests
            let result: UnbondRequestsResponse = self.query_unbond_requests(deps)?;
            let mut unbonding = Uint128::zero();

            for (id, amount) in result.requests {
                // query the info about one item
                let all_history: AllHistoryResponse = self.query_single_history(deps, id)?;

                let (released, exchange_rate) = if all_history.history.is_empty() {
                    // if nothing returned, the request is not yet started, so we estimate exchange_rate * amount
                    let state = self.cached_state(deps)?;
                    (false, state.exchange_rate)
                } else {
                    let batch = all_history.history.first().unwrap();
                    if batch.batch_id == id {
                        (batch.released, batch.withdraw_rate)
                    } else {
                        Err(adapter_error_empty("prism", "wrong_id"))?
                    }
                };

                if !released {
                    unbonding += exchange_rate * amount;
                }
            }

            self.unbonding_cache = Some(unbonding);
        }

        Ok(self.unbonding_cache.unwrap())
    }

    fn cached_state(&mut self, deps: &Deps) -> CustomResult<&StateResponse> {
        if self.state_cache.is_none() {
            let result = self.query_state(deps)?;
            self.state_cache = Some(result);
        }

        Ok(self.state_cache.as_ref().unwrap())
    }
}

impl LsdAdapter for Prism {
    fn get_name(&self) -> &str {
        "prism"
    }

    fn asset(&self) -> AssetInfo {
        token_asset_info(self.cw20.clone())
    }

    fn unbond(&self, _deps: &Deps, amount: Uint128) -> CustomResult<Vec<CosmosMsg>> {
        Ok(vec![self.get_unbond_msg(amount)?])
    }

    fn query_unbonding(&mut self, deps: &Deps) -> CustomResult<Uint128> {
        // cache is not really needed here
        self.cached_unbonding(deps)
    }

    fn withdraw(&mut self, _deps: &Deps, _amount: Uint128) -> CustomResult<Vec<CosmosMsg>> {
        Ok(vec![self.get_withdraw_msg()?])
    }

    fn query_withdrawable(&mut self, deps: &Deps) -> CustomResult<Uint128> {
        let result: WithdrawableUnbondedResponse = self.query_withdrawable_unbonded(deps)?;
        Ok(result.withdrawable)
    }

    fn query_factor_x_to_normal(&mut self, deps: &Deps) -> CustomResult<Decimal> {
        Ok(self.cached_state(deps)?.exchange_rate)
    }
}
