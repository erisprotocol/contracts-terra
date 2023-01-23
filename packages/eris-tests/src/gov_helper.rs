use anyhow::Result;
use cosmwasm_std::{
    attr, coin, to_binary, Addr, Delegation, FullDelegation, StdResult, Uint128, VoteOption,
};
use cw20::Cw20ExecuteMsg;
use cw_multi_test::{App, AppResponse, Executor};
use eris::{emp_gauges::AddEmpInfo, governance_helper::WEEK};

use crate::base::{BaseErisTestInitMessage, BaseErisTestPackage};

pub const MULTIPLIER: u64 = 1000000;

pub struct EscrowHelper {
    pub owner: Addr,
    pub base: BaseErisTestPackage,
}

impl EscrowHelper {
    pub fn init(router_ref: &mut App, use_default_hub: bool) -> Self {
        let owner = Addr::unchecked("owner");
        Self {
            owner: owner.clone(),
            base: BaseErisTestPackage::init_all(
                router_ref,
                BaseErisTestInitMessage {
                    owner,
                    use_default_hub,
                },
            ),
        }
    }

    pub fn emp_tune(&self, router_ref: &mut App) -> Result<AppResponse> {
        router_ref.execute_contract(
            self.owner.clone(),
            self.base.emp_gauges.get_address(),
            &eris::emp_gauges::ExecuteMsg::TuneEmps {},
            &[],
        )
    }

    pub fn emp_add_points(
        &self,
        router_ref: &mut App,
        emps: Vec<AddEmpInfo>,
    ) -> Result<AppResponse> {
        self.emp_execute(
            router_ref,
            eris::emp_gauges::ExecuteMsg::AddEmps {
                emps,
            },
        )
    }

    pub fn emp_execute(
        &self,
        router_ref: &mut App,
        execute: eris::emp_gauges::ExecuteMsg,
    ) -> Result<AppResponse> {
        router_ref.execute_contract(
            self.owner.clone(),
            self.base.emp_gauges.get_address(),
            &execute,
            &[],
        )
    }

    pub fn emp_execute_sender(
        &self,
        router_ref: &mut App,
        execute: eris::emp_gauges::ExecuteMsg,
        sender: impl Into<String>,
    ) -> Result<AppResponse> {
        router_ref.execute_contract(
            Addr::unchecked(sender),
            self.base.emp_gauges.get_address(),
            &execute,
            &[],
        )
    }

    pub fn emp_query_config(
        &self,
        router_ref: &mut App,
    ) -> StdResult<eris::emp_gauges::ConfigResponse> {
        router_ref.wrap().query_wasm_smart(
            self.base.emp_gauges.get_address_string(),
            &eris::emp_gauges::QueryMsg::Config {},
        )
    }

    pub fn emp_query_validator_history(
        &self,
        router_ref: &mut App,
        validator_addr: impl Into<String>,
        period: u64,
    ) -> StdResult<eris::emp_gauges::VotedValidatorInfoResponse> {
        router_ref.wrap().query_wasm_smart(
            self.base.emp_gauges.get_address_string(),
            &eris::emp_gauges::QueryMsg::ValidatorInfoAtPeriod {
                validator_addr: validator_addr.into(),
                period,
            },
        )
    }

    pub fn emp_query_tune_info(
        &self,
        router_ref: &mut App,
    ) -> StdResult<eris::emp_gauges::GaugeInfoResponse> {
        router_ref.wrap().query_wasm_smart(
            self.base.emp_gauges.get_address_string(),
            &eris::emp_gauges::QueryMsg::TuneInfo {},
        )
    }

    pub fn amp_tune(&self, router_ref: &mut App) -> Result<AppResponse> {
        router_ref.execute_contract(
            self.owner.clone(),
            self.base.amp_gauges.get_address(),
            &eris::amp_gauges::ExecuteMsg::TuneVamp {},
            &[],
        )
    }

    pub fn amp_vote(
        &self,
        router_ref: &mut App,
        user: impl Into<String>,
        votes: Vec<(String, u16)>,
    ) -> Result<AppResponse> {
        self.amp_execute_sender(
            router_ref,
            eris::amp_gauges::ExecuteMsg::Vote {
                votes,
            },
            user,
        )
    }

    pub fn amp_execute(
        &self,
        router_ref: &mut App,
        execute: eris::amp_gauges::ExecuteMsg,
    ) -> Result<AppResponse> {
        router_ref.execute_contract(
            self.owner.clone(),
            self.base.amp_gauges.get_address(),
            &execute,
            &[],
        )
    }

    pub fn amp_execute_sender(
        &self,
        router_ref: &mut App,
        execute: eris::amp_gauges::ExecuteMsg,
        sender: impl Into<String>,
    ) -> Result<AppResponse> {
        router_ref.execute_contract(
            Addr::unchecked(sender),
            self.base.amp_gauges.get_address(),
            &execute,
            &[],
        )
    }

    pub fn amp_query_config(
        &self,
        router_ref: &mut App,
    ) -> StdResult<eris::amp_gauges::ConfigResponse> {
        router_ref.wrap().query_wasm_smart(
            self.base.amp_gauges.get_address_string(),
            &eris::amp_gauges::QueryMsg::Config {},
        )
    }

    pub fn amp_query_validator_history(
        &self,
        router_ref: &mut App,
        validator_addr: impl Into<String>,
        period: u64,
    ) -> StdResult<eris::amp_gauges::VotedValidatorInfoResponse> {
        router_ref.wrap().query_wasm_smart(
            self.base.amp_gauges.get_address_string(),
            &eris::amp_gauges::QueryMsg::ValidatorInfoAtPeriod {
                validator_addr: validator_addr.into(),
                period,
            },
        )
    }

    pub fn amp_query_tune_info(
        &self,
        router_ref: &mut App,
    ) -> StdResult<eris::amp_gauges::GaugeInfoResponse> {
        router_ref.wrap().query_wasm_smart(
            self.base.amp_gauges.get_address_string(),
            &eris::amp_gauges::QueryMsg::TuneInfo {},
        )
    }

    pub fn mint_amp_lp(&self, router_ref: &mut App, to: String, amount: u128) {
        let msg = cw20::Cw20ExecuteMsg::Mint {
            recipient: to.clone(),
            amount: Uint128::from(amount),
        };
        let res = router_ref
            .execute_contract(self.owner.clone(), self.base.amp_lp.get_address(), &msg, &[])
            .unwrap();
        assert_eq!(res.events[1].attributes[1], attr("action", "mint"));
        assert_eq!(res.events[1].attributes[2], attr("to", to));
        assert_eq!(res.events[1].attributes[3], attr("amount", Uint128::from(amount)));
    }

    pub fn ve_lock_lp(
        &self,
        router_ref: &mut App,
        sender: impl Into<String>,
        amount: u128,
        lock_time: u64,
    ) -> Result<AppResponse> {
        let sender: String = sender.into();
        self.mint_amp_lp(router_ref, sender.clone(), amount);

        let cw20msg = Cw20ExecuteMsg::Send {
            contract: self.base.voting_escrow.get_address_string(),
            amount: Uint128::from(amount),
            msg: to_binary(&eris::voting_escrow::Cw20HookMsg::CreateLock {
                time: lock_time,
            })
            .unwrap(),
        };
        router_ref.execute_contract(
            Addr::unchecked(sender),
            self.base.amp_lp.get_address(),
            &cw20msg,
            &[],
        )
    }

    pub fn ve_add_funds_lock(
        &self,
        router_ref: &mut App,
        sender: impl Into<String>,
        amount: u128,
        extend_to_min_periods: Option<bool>,
    ) -> Result<AppResponse> {
        let sender: String = sender.into();
        self.mint_amp_lp(router_ref, sender.clone(), amount);

        let cw20msg = Cw20ExecuteMsg::Send {
            contract: self.base.voting_escrow.get_address_string(),
            amount: Uint128::from(amount),
            msg: to_binary(&eris::voting_escrow::Cw20HookMsg::ExtendLockAmount {
                extend_to_min_periods,
            })
            .unwrap(),
        };
        router_ref.execute_contract(
            Addr::unchecked(sender),
            self.base.amp_lp.get_address(),
            &cw20msg,
            &[],
        )
    }

    pub fn ve_add_funds_lock_for_user(
        &self,
        router_ref: &mut App,
        sender: impl Into<String>,
        user: impl Into<String>,
        amount: u128,
    ) -> Result<AppResponse> {
        let sender: String = sender.into();
        self.mint_amp_lp(router_ref, sender.clone(), amount);

        let cw20msg = Cw20ExecuteMsg::Send {
            contract: self.base.voting_escrow.get_address_string(),
            amount: Uint128::from(amount),
            msg: to_binary(&eris::voting_escrow::Cw20HookMsg::DepositFor {
                user: user.into(),
            })
            .unwrap(),
        };
        router_ref.execute_contract(
            Addr::unchecked(sender),
            self.base.amp_lp.get_address(),
            &cw20msg,
            &[],
        )
    }

    pub fn ve_extend_lock_time(
        &self,
        router_ref: &mut App,
        sender: impl Into<String>,
        periods: u64,
    ) -> Result<AppResponse> {
        self.ve_execute_sender(
            router_ref,
            eris::voting_escrow::ExecuteMsg::ExtendLockTime {
                time: periods * WEEK,
            },
            Addr::unchecked(sender),
        )
    }

    pub fn ve_withdraw(
        &self,
        router_ref: &mut App,
        sender: impl Into<String>,
    ) -> Result<AppResponse> {
        self.ve_execute_sender(
            router_ref,
            eris::voting_escrow::ExecuteMsg::Withdraw {},
            Addr::unchecked(sender),
        )
    }

    pub fn ve_execute(
        &self,
        router_ref: &mut App,
        execute: eris::voting_escrow::ExecuteMsg,
    ) -> Result<AppResponse> {
        router_ref.execute_contract(
            self.owner.clone(),
            self.base.voting_escrow.get_address(),
            &execute,
            &[],
        )
    }

    pub fn ve_execute_sender(
        &self,
        router_ref: &mut App,
        execute: eris::voting_escrow::ExecuteMsg,
        sender: Addr,
    ) -> Result<AppResponse> {
        router_ref.execute_contract(sender, self.base.voting_escrow.get_address(), &execute, &[])
    }

    pub fn hub_execute(
        &self,
        router_ref: &mut App,
        execute: eris::hub::ExecuteMsg,
    ) -> Result<AppResponse> {
        router_ref.execute_contract(self.owner.clone(), self.base.hub.get_address(), &execute, &[])
    }

    pub fn hub_remove_validator(
        &self,
        router_ref: &mut App,
        validator_addr: impl Into<String>,
    ) -> Result<AppResponse> {
        self.hub_execute(
            router_ref,
            eris::hub::ExecuteMsg::RemoveValidator {
                validator: validator_addr.into(),
            },
        )
    }

    pub fn hub_harvest(&self, router_ref: &mut App) -> Result<AppResponse> {
        self.hub_execute(router_ref, eris::hub::ExecuteMsg::Harvest {})
    }

    pub fn hub_add_validator(
        &self,
        router_ref: &mut App,
        validator_addr: impl Into<String>,
    ) -> Result<AppResponse> {
        self.hub_execute(
            router_ref,
            eris::hub::ExecuteMsg::AddValidator {
                validator: validator_addr.into(),
            },
        )
    }

    pub fn hub_bond(
        &self,
        router_ref: &mut App,
        sender: impl Into<String>,
        amount: u128,
        denom: impl Into<String>,
    ) -> Result<AppResponse> {
        router_ref.execute_contract(
            Addr::unchecked(sender),
            self.base.hub.get_address(),
            &eris::hub::ExecuteMsg::Bond {
                receiver: None,
            },
            &[coin(amount, denom.into())],
        )
    }

    pub fn hub_execute_sender(
        &self,
        router_ref: &mut App,
        execute: eris::hub::ExecuteMsg,
        sender: Addr,
    ) -> Result<AppResponse> {
        router_ref.execute_contract(sender, self.base.hub.get_address(), &execute, &[])
    }

    pub fn hub_rebalance(&self, router_ref: &mut App) -> Result<AppResponse> {
        self.hub_execute(
            router_ref,
            eris::hub::ExecuteMsg::Rebalance {
                min_redelegation: None,
            },
        )
    }

    pub fn hub_tune(&self, router_ref: &mut App) -> Result<AppResponse> {
        self.hub_execute(router_ref, eris::hub::ExecuteMsg::TuneDelegations {})
    }

    pub fn hub_query_config(&self, router_ref: &mut App) -> StdResult<eris::hub::ConfigResponse> {
        router_ref
            .wrap()
            .query_wasm_smart(self.base.hub.get_address_string(), &eris::hub::QueryMsg::Config {})
    }

    pub fn hub_query_wanted_delegations(
        &self,
        router_ref: &mut App,
    ) -> StdResult<eris::hub::WantedDelegationsResponse> {
        router_ref.wrap().query_wasm_smart(
            self.base.hub.get_address_string(),
            &eris::hub::QueryMsg::WantedDelegations {},
        )
    }

    pub fn hub_query_state(&self, router_ref: &mut App) -> StdResult<eris::hub::StateResponse> {
        router_ref
            .wrap()
            .query_wasm_smart(self.base.hub.get_address_string(), &eris::hub::QueryMsg::State {})
    }

    pub fn hub_query_all_delegations(&self, router_ref: &mut App) -> StdResult<Vec<Delegation>> {
        router_ref.wrap().query_all_delegations(self.base.hub.get_address_string())
    }

    pub fn hub_query_delegation(
        &self,
        router_ref: &mut App,
        validator: impl Into<String>,
    ) -> StdResult<Option<FullDelegation>> {
        router_ref.wrap().query_delegation(self.base.hub.get_address_string(), validator)
    }

    pub fn prop_vote(
        &self,
        router_ref: &mut App,
        user: impl Into<String>,
        proposal_id: u64,
        vote: VoteOption,
    ) -> Result<AppResponse> {
        self.prop_execute_sender(
            router_ref,
            eris::prop_gauges::ExecuteMsg::Vote {
                proposal_id,
                vote,
            },
            user,
        )
    }

    pub fn prop_init(
        &self,
        router_ref: &mut App,
        user: impl Into<String>,
        proposal_id: u64,
        end_time_s: u64,
    ) -> Result<AppResponse> {
        self.prop_execute_sender(
            router_ref,
            eris::prop_gauges::ExecuteMsg::InitProp {
                proposal_id,
                end_time_s,
            },
            user,
        )
    }

    pub fn prop_execute(
        &self,
        router_ref: &mut App,
        execute: eris::prop_gauges::ExecuteMsg,
    ) -> Result<AppResponse> {
        router_ref.execute_contract(
            self.owner.clone(),
            self.base.prop_gauges.get_address(),
            &execute,
            &[],
        )
    }

    pub fn prop_execute_sender(
        &self,
        router_ref: &mut App,
        execute: eris::prop_gauges::ExecuteMsg,
        sender: impl Into<String>,
    ) -> Result<AppResponse> {
        router_ref.execute_contract(
            Addr::unchecked(sender),
            self.base.prop_gauges.get_address(),
            &execute,
            &[],
        )
    }

    pub fn prop_query_config(
        &self,
        router_ref: &mut App,
    ) -> StdResult<eris::prop_gauges::ConfigResponse> {
        router_ref.wrap().query_wasm_smart(
            self.base.prop_gauges.get_address_string(),
            &eris::prop_gauges::QueryMsg::Config {},
        )
    }
}
