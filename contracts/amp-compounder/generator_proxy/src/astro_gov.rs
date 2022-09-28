use cosmwasm_std::{Addr, Api, CosmosMsg, QuerierWrapper, StdError, StdResult, to_binary, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;
use cw_storage_plus::{Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use astroport::asset::addr_validate_to_lower;
use astroport_governance::escrow_fee_distributor::{ExecuteMsg as FeeExecuteMsg};
use astroport_governance::generator_controller::{ExecuteMsg as ControllerExecuteMsg};
use astroport_governance::voting_escrow::{Cw20HookMsg as VotingCw20HookMsg, ExecuteMsg as VotingExecuteMsg, QueryMsg as VotingQueryMsg, LockInfoResponse, VotingPowerResponse};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AstroGovBase<T> {
    pub fee_distributor: T,
    pub generator_controller: T,
    pub voting_escrow: T,
    pub xastro_token: T,
}

pub type AstroGovUnchecked = AstroGovBase<String>;
pub type AstroGov = AstroGovBase<Addr>;

impl From<AstroGov> for AstroGovUnchecked {
    fn from(governance: AstroGov) -> Self {
        AstroGovUnchecked {
            fee_distributor: governance.fee_distributor.to_string(),
            generator_controller: governance.generator_controller.to_string(),
            voting_escrow: governance.voting_escrow.to_string(),
            xastro_token: governance.xastro_token.to_string(),
        }
    }
}

impl AstroGovUnchecked {
    pub fn check(&self, api: &dyn Api) -> StdResult<AstroGov> {
        Ok(AstroGov {
            fee_distributor: addr_validate_to_lower(api, &self.fee_distributor)?,
            generator_controller: addr_validate_to_lower(api, &self.generator_controller)?,
            voting_escrow: addr_validate_to_lower(api, &self.voting_escrow)?,
            xastro_token: addr_validate_to_lower(api, &self.xastro_token)?,
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct Lock {
    pub amount: Uint128,
    pub start: u64,
    pub end: u64,
    pub last_extend_lock_period: u64,
}

pub const LOCK: Map<Addr, Lock> = Map::new("locked");
pub const REWARDS_PER_WEEK: Map<u64, Uint128> = Map::new("rewards_per_week");
pub const LAST_CLAIM_PERIOD: Map<Addr, u64> = Map::new("last_claim_period");

impl AstroGov {

    pub fn query_lock(&self, querier: &QuerierWrapper, user: Addr) -> StdResult<Lock> {
        let lock = LOCK.query(querier, self.voting_escrow.clone(), user)?;
        Ok(lock.unwrap_or_default())
    }

    pub fn query_last_claim_period(&self, querier: &QuerierWrapper, user: Addr) -> StdResult<u64> {
        let last_claim_period = LAST_CLAIM_PERIOD.query(querier, self.fee_distributor.clone(), user)?;
        last_claim_period.ok_or_else(|| StdError::generic_err("last_claim_period not found"))
    }

    pub fn claim_msg(
        &self,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.fee_distributor.to_string(),
            msg: to_binary(&FeeExecuteMsg::Claim {
                recipient: None,
                max_periods: None,
            })?,
            funds: vec![],
        }))
    }

    pub fn controller_vote_msg(
        &self,
        votes: Vec<(String, u16)>,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.generator_controller.to_string(),
            msg: to_binary(&ControllerExecuteMsg::Vote {
                votes,
            })?,
            funds: vec![],
        }))
    }

    pub fn create_lock_msg(
        &self,
        amount: Uint128,
        time: u64,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.xastro_token.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: self.voting_escrow.to_string(),
                amount,
                msg: to_binary(&VotingCw20HookMsg::CreateLock {
                    time,
                })?,
            })?,
            funds: vec![],
        }))
    }

    pub fn extend_lock_amount_msg(
        &self,
        amount: Uint128,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.xastro_token.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: self.voting_escrow.to_string(),
                amount,
                msg: to_binary(&VotingCw20HookMsg::ExtendLockAmount {})?,
            })?,
            funds: vec![],
        }))
    }

    pub fn extend_lock_time_msg(
        &self,
        time: u64,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.voting_escrow.to_string(),
            msg: to_binary(&VotingExecuteMsg::ExtendLockTime {
                time,
            })?,
            funds: vec![],
        }))
    }

    pub fn withdraw_msg(
        &self,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.voting_escrow.to_string(),
            msg: to_binary(&VotingExecuteMsg::Withdraw {})?,
            funds: vec![],
        }))
    }

    // from astroport-governance\contracts\escrow_fee_distributor\src\contract.rs
    pub fn calc_claim_amount(&self, querier: &QuerierWrapper, account: Addr, claim_start: u64, current_period: u64) -> StdResult<Uint128> {
        let user_lock_info: LockInfoResponse = querier.query_wasm_smart(
            &self.voting_escrow,
            &VotingQueryMsg::LockInfo {
                user: account.to_string(),
            },
        )?;

        let mut claim_period = claim_start;

        let lock_end_period = user_lock_info.end;
        let mut claim_amount: Uint128 = Default::default();

        loop {
            // User cannot claim for the current period
            if claim_period >= current_period {
                break;
            }

            // User cannot claim past their max lock period
            if claim_period > lock_end_period {
                break;
            }

            let user_voting_power: VotingPowerResponse = querier.query_wasm_smart(
                &self.voting_escrow,
                &VotingQueryMsg::UserVotingPowerAtPeriod {
                    user: account.to_string(),
                    period: claim_period,
                },
            )?;

            let total_voting_power: VotingPowerResponse = querier.query_wasm_smart(
                &self.voting_escrow,
                &VotingQueryMsg::TotalVotingPowerAtPeriod {
                    period: claim_period,
                },
            )?;

            if !user_voting_power.voting_power.is_zero() && !total_voting_power.voting_power.is_zero() {
                claim_amount = claim_amount.checked_add(self.calculate_reward(
                    querier,
                    claim_period,
                    user_voting_power.voting_power,
                    total_voting_power.voting_power,
                )?)?;
            }

            claim_period += 1;
        }

        Ok(claim_amount)
    }

    fn calculate_reward(
        &self,
        querier: &QuerierWrapper,
        period: u64,
        user_vp: Uint128,
        total_vp: Uint128,
    ) -> StdResult<Uint128> {
        let rewards_per_week = REWARDS_PER_WEEK
            .query(querier, self.fee_distributor.clone(), period)?
            .unwrap_or_default();

        Ok(user_vp.multiply_ratio(rewards_per_week, total_vp))
    }
}
