use anyhow::Result;
use cosmwasm_std::{attr, coin, to_binary, Addr, Delegation, StdResult, Uint128};
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
    pub fn init(router_ref: &mut App) -> Self {
        let owner = Addr::unchecked("owner");
        Self {
            owner: owner.clone(),
            base: BaseErisTestPackage::init_all(
                router_ref,
                BaseErisTestInitMessage {
                    owner,
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
    ) -> Result<AppResponse> {
        let sender: String = sender.into();
        self.mint_amp_lp(router_ref, sender.clone(), amount);

        let cw20msg = Cw20ExecuteMsg::Send {
            contract: self.base.voting_escrow.get_address_string(),
            amount: Uint128::from(amount),
            msg: to_binary(&eris::voting_escrow::Cw20HookMsg::ExtendLockAmount {}).unwrap(),
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

    pub fn hub_query_all_delegations(&self, router_ref: &mut App) -> StdResult<Vec<Delegation>> {
        router_ref.wrap().query_all_delegations(self.base.hub.get_address_string())
    }

    // pub fn init(router_ref: &mut App, owner: Addr) -> Self {
    //     let astro_token_contract = Box::new(ContractWrapper::new_with_empty(
    //         astroport_token::contract::execute,
    //         astroport_token::contract::instantiate,
    //         astroport_token::contract::query,
    //     ));

    //     let astro_token_code_id = router_ref.store_code(astro_token_contract);

    //     let msg = astro::InstantiateMsg {
    //         name: String::from("Astro token"),
    //         symbol: String::from("ASTRO"),
    //         decimals: 6,
    //         initial_balances: vec![],
    //         mint: Some(MinterResponse {
    //             minter: owner.to_string(),
    //             cap: None,
    //         }),
    //         marketing: None,
    //     };

    //     let astro_token = router_ref
    //         .instantiate_contract(
    //             astro_token_code_id,
    //             owner.clone(),
    //             &msg,
    //             &[],
    //             String::from("ASTRO"),
    //             None,
    //         )
    //         .unwrap();

    //     let staking_contract = Box::new(
    //         ContractWrapper::new_with_empty(
    //             astroport_staking::contract::execute,
    //             astroport_staking::contract::instantiate,
    //             astroport_staking::contract::query,
    //         )
    //         .with_reply_empty(astroport_staking::contract::reply),
    //     );

    //     let staking_code_id = router_ref.store_code(staking_contract);

    //     let msg = xastro::InstantiateMsg {
    //         owner: owner.to_string(),
    //         token_code_id: astro_token_code_id,
    //         deposit_token_addr: astro_token.to_string(),
    //         marketing: None,
    //     };
    //     let staking_instance = router_ref
    //         .instantiate_contract(
    //             staking_code_id,
    //             owner.clone(),
    //             &msg,
    //             &[],
    //             String::from("xASTRO"),
    //             None,
    //         )
    //         .unwrap();

    //     let res = router_ref
    //         .wrap()
    //         .query::<xastro::ConfigResponse>(&QueryRequest::Wasm(WasmQuery::Smart {
    //             contract_addr: staking_instance.to_string(),
    //             msg: to_binary(&xastro::QueryMsg::Config {}).unwrap(),
    //         }))
    //         .unwrap();

    //     let voting_contract = Box::new(ContractWrapper::new_with_empty(
    //         voting_escrow::contract::execute,
    //         voting_escrow::contract::instantiate,
    //         voting_escrow::contract::query,
    //     ));

    //     let voting_code_id = router_ref.store_code(voting_contract);

    //     let msg = InstantiateMsg {
    //         owner: owner.to_string(),
    //         guardian_addr: Some("guardian".to_string()),
    //         deposit_token_addr: res.share_token_addr.to_string(),
    //         marketing: None,
    //         logo_urls_whitelist: vec![],
    //     };
    //     let voting_instance = router_ref
    //         .instantiate_contract(
    //             voting_code_id,
    //             owner.clone(),
    //             &msg,
    //             &[],
    //             String::from("vxASTRO"),
    //             None,
    //         )
    //         .unwrap();

    //     Self {
    //         owner,
    //         xastro_token: res.share_token_addr,
    //         astro_token,
    //         staking_instance,
    //         escrow_instance: voting_instance,
    //         astro_token_code_id,
    //     }
    // }

    // pub fn mint_xastro(&self, router_ref: &mut App, to: &str, amount: u64) {
    //     let amount = amount * MULTIPLIER;
    //     let msg = Cw20ExecuteMsg::Mint {
    //         recipient: String::from(to),
    //         amount: Uint128::from(amount),
    //     };
    //     let res = router_ref
    //         .execute_contract(self.owner.clone(), self.astro_token.clone(), &msg, &[])
    //         .unwrap();
    //     assert_eq!(res.events[1].attributes[1], attr("action", "mint"));
    //     assert_eq!(res.events[1].attributes[2], attr("to", String::from(to)));
    //     assert_eq!(
    //         res.events[1].attributes[3],
    //         attr("amount", Uint128::from(amount))
    //     );

    //     let to_addr = Addr::unchecked(to);
    //     let msg = Cw20ExecuteMsg::Send {
    //         contract: self.staking_instance.to_string(),
    //         msg: to_binary(&xastro::Cw20HookMsg::Enter {}).unwrap(),
    //         amount: Uint128::from(amount),
    //     };
    //     router_ref
    //         .execute_contract(to_addr, self.astro_token.clone(), &msg, &[])
    //         .unwrap();
    // }

    // pub fn check_xastro_balance(&self, router_ref: &mut App, user: &str, amount: u64) {
    //     let amount = amount * MULTIPLIER;
    //     let res: BalanceResponse = router_ref
    //         .wrap()
    //         .query_wasm_smart(
    //             self.xastro_token.clone(),
    //             &Cw20QueryMsg::Balance {
    //                 address: user.to_string(),
    //             },
    //         )
    //         .unwrap();
    //     assert_eq!(res.balance.u128(), amount as u128);
    // }

    // pub fn create_lock(
    //     &self,
    //     router_ref: &mut App,
    //     user: &str,
    //     time: u64,
    //     amount: f32,
    // ) -> Result<AppResponse> {
    //     let amount = (amount * MULTIPLIER as f32) as u64;
    //     let cw20msg = Cw20ExecuteMsg::Send {
    //         contract: self.escrow_instance.to_string(),
    //         amount: Uint128::from(amount),
    //         msg: to_binary(&Cw20HookMsg::CreateLock { time }).unwrap(),
    //     };
    //     router_ref.execute_contract(
    //         Addr::unchecked(user),
    //         self.xastro_token.clone(),
    //         &cw20msg,
    //         &[],
    //     )
    // }

    // pub fn extend_lock_amount(
    //     &self,
    //     router_ref: &mut App,
    //     user: &str,
    //     amount: f32,
    // ) -> Result<AppResponse> {
    //     let amount = (amount * MULTIPLIER as f32) as u64;
    //     let cw20msg = Cw20ExecuteMsg::Send {
    //         contract: self.escrow_instance.to_string(),
    //         amount: Uint128::from(amount),
    //         msg: to_binary(&Cw20HookMsg::ExtendLockAmount {}).unwrap(),
    //     };
    //     router_ref.execute_contract(
    //         Addr::unchecked(user),
    //         self.xastro_token.clone(),
    //         &cw20msg,
    //         &[],
    //     )
    // }

    // pub fn deposit_for(
    //     &self,
    //     router_ref: &mut App,
    //     from: &str,
    //     to: &str,
    //     amount: f32,
    // ) -> Result<AppResponse> {
    //     let amount = (amount * MULTIPLIER as f32) as u64;
    //     let cw20msg = Cw20ExecuteMsg::Send {
    //         contract: self.escrow_instance.to_string(),
    //         amount: Uint128::from(amount),
    //         msg: to_binary(&Cw20HookMsg::DepositFor {
    //             user: to.to_string(),
    //         })
    //         .unwrap(),
    //     };
    //     router_ref.execute_contract(
    //         Addr::unchecked(from),
    //         self.xastro_token.clone(),
    //         &cw20msg,
    //         &[],
    //     )
    // }

    // pub fn extend_lock_time(&self, router_ref: &mut App, user: &str, time: u64) -> Result<AppResponse> {
    //     router_ref.execute_contract(
    //         Addr::unchecked(user),
    //         self.escrow_instance.clone(),
    //         &ExecuteMsg::ExtendLockTime { time },
    //         &[],
    //     )
    // }

    // pub fn withdraw(&self, router_ref: &mut App, user: &str) -> Result<AppResponse> {
    //     router_ref.execute_contract(
    //         Addr::unchecked(user),
    //         self.escrow_instance.clone(),
    //         &ExecuteMsg::Withdraw {},
    //         &[],
    //     )
    // }

    // pub fn update_blacklist(
    //     &self,
    //     router_ref: &mut App,
    //     append_addrs: Option<Vec<String>>,
    //     remove_addrs: Option<Vec<String>>,
    // ) -> Result<AppResponse> {
    //     router_ref.execute_contract(
    //         Addr::unchecked("owner"),
    //         self.escrow_instance.clone(),
    //         &ExecuteMsg::UpdateBlacklist {
    //             append_addrs,
    //             remove_addrs,
    //         },
    //         &[],
    //     )
    // }

    // pub fn query_user_vp(&self, router_ref: &mut App, user: &str) -> StdResult<f32> {
    //     router_ref
    //         .wrap()
    //         .query_wasm_smart(
    //             self.escrow_instance.clone(),
    //             &QueryMsg::UserVotingPower {
    //                 user: user.to_string(),
    //             },
    //         )
    //         .map(|vp: VotingPowerResponse| vp.voting_power.u128() as f32 / MULTIPLIER as f32)
    // }

    // pub fn query_user_vp_at(&self, router_ref: &mut App, user: &str, time: u64) -> StdResult<f32> {
    //     router_ref
    //         .wrap()
    //         .query_wasm_smart(
    //             self.escrow_instance.clone(),
    //             &QueryMsg::UserVotingPowerAt {
    //                 user: user.to_string(),
    //                 time,
    //             },
    //         )
    //         .map(|vp: VotingPowerResponse| vp.voting_power.u128() as f32 / MULTIPLIER as f32)
    // }

    // pub fn query_user_vp_at_period(
    //     &self,
    //     router_ref: &mut App,
    //     user: &str,
    //     period: u64,
    // ) -> StdResult<f32> {
    //     router_ref
    //         .wrap()
    //         .query_wasm_smart(
    //             self.escrow_instance.clone(),
    //             &QueryMsg::UserVotingPowerAtPeriod {
    //                 user: user.to_string(),
    //                 period,
    //             },
    //         )
    //         .map(|vp: VotingPowerResponse| vp.voting_power.u128() as f32 / MULTIPLIER as f32)
    // }

    // pub fn query_total_vp(&self, router_ref: &mut App) -> StdResult<f32> {
    //     router_ref
    //         .wrap()
    //         .query_wasm_smart(self.escrow_instance.clone(), &QueryMsg::TotalVotingPower {})
    //         .map(|vp: VotingPowerResponse| vp.voting_power.u128() as f32 / MULTIPLIER as f32)
    // }

    // pub fn query_total_vp_at(&self, router_ref: &mut App, time: u64) -> StdResult<f32> {
    //     router_ref
    //         .wrap()
    //         .query_wasm_smart(
    //             self.escrow_instance.clone(),
    //             &QueryMsg::TotalVotingPowerAt { time },
    //         )
    //         .map(|vp: VotingPowerResponse| vp.voting_power.u128() as f32 / MULTIPLIER as f32)
    // }

    // pub fn query_total_vp_at_period(&self, router_ref: &mut App, period: u64) -> StdResult<f32> {
    //     router_ref
    //         .wrap()
    //         .query_wasm_smart(
    //             self.escrow_instance.clone(),
    //             &QueryMsg::TotalVotingPowerAtPeriod { period },
    //         )
    //         .map(|vp: VotingPowerResponse| vp.voting_power.u128() as f32 / MULTIPLIER as f32)
    // }

    // pub fn query_lock_info(&self, router_ref: &mut App, user: &str) -> StdResult<LockInfoResponse> {
    //     router_ref.wrap().query_wasm_smart(
    //         self.escrow_instance.clone(),
    //         &QueryMsg::LockInfo {
    //             user: user.to_string(),
    //         },
    //     )
    // }
}
