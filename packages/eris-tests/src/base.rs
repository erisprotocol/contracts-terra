use cosmwasm_schema::cw_serde;

use cosmwasm_std::{attr, Addr, Decimal, StdResult, Uint128};
use cw20::{BalanceResponse, Cw20QueryMsg, MinterResponse};

use cw_multi_test::{App, ContractWrapper, Executor};

pub const MULTIPLIER: u64 = 1_000_000;

#[cw_serde]
pub struct ContractInfo {
    pub address: Addr,
    pub code_id: u64,
}

#[cw_serde]
pub struct ContractInfoWrapper {
    contract: Option<ContractInfo>,
}

impl ContractInfoWrapper {
    pub fn get_address_string(&self) -> String {
        self.contract.clone().unwrap().address.to_string()
    }
    pub fn get_address(&self) -> Addr {
        self.contract.clone().unwrap().address
    }
}

impl From<Option<ContractInfo>> for ContractInfoWrapper {
    fn from(item: Option<ContractInfo>) -> Self {
        ContractInfoWrapper {
            contract: item,
        }
    }
}

#[cw_serde]
pub struct BaseErisTestPackage {
    pub owner: Addr,
    pub token_id: Option<u64>,
    pub hub: ContractInfoWrapper,
    pub voting_escrow: ContractInfoWrapper,
    pub emp_gauges: ContractInfoWrapper,
    pub amp_gauges: ContractInfoWrapper,
    pub amp_lp: ContractInfoWrapper,
}

#[cw_serde]
pub struct BaseErisTestInitMessage {
    pub owner: Addr,
}

impl BaseErisTestPackage {
    pub fn init_all(router: &mut App, msg: BaseErisTestInitMessage) -> Self {
        let mut base_pack = BaseErisTestPackage {
            owner: msg.owner.clone(),
            token_id: None,
            voting_escrow: None.into(),
            hub: None.into(),
            amp_lp: None.into(),
            emp_gauges: None.into(),
            amp_gauges: None.into(),
        };

        base_pack.init_token(router, msg.owner.clone());
        base_pack.init_hub(router, msg.owner.clone());
        base_pack.init_voting_escrow(router, msg.owner.clone());
        base_pack.init_emp_registry(router, msg.owner.clone());
        base_pack.init_amp_gauges(router, msg.owner.clone());

        base_pack.init_hub_delegation_strategy(router, msg.owner);

        base_pack
    }

    fn init_token(&mut self, router: &mut App, owner: Addr) {
        let contract = Box::new(ContractWrapper::new_with_empty(
            eris_staking_token::execute,
            eris_staking_token::instantiate,
            eris_staking_token::query,
        ));

        let token_code_id = router.store_code(contract);

        self.token_id = Some(token_code_id);

        let init_msg = cw20_base::msg::InstantiateMsg {
            name: "ampLP".to_string(),
            symbol: "stake".to_string(),
            decimals: 6,
            initial_balances: vec![],
            mint: Some(MinterResponse {
                minter: owner.to_string(),
                cap: None,
            }),
            marketing: None,
        };

        let instance = router
            .instantiate_contract(self.token_id.unwrap(), owner, &init_msg, &[], "Hub", None)
            .unwrap();

        self.amp_lp = Some(ContractInfo {
            address: instance,
            code_id: self.token_id.unwrap(),
        })
        .into()
    }

    fn init_hub(&mut self, router: &mut App, owner: Addr) {
        let hub_contract = Box::new(
            ContractWrapper::new_with_empty(
                eris_staking_hub::contract::execute,
                eris_staking_hub::contract::instantiate,
                eris_staking_hub::contract::query,
            )
            .with_reply(eris_staking_hub::contract::reply),
        );

        let code_id = router.store_code(hub_contract);

        let init_msg = eris::hub::InstantiateMsg {
            cw20_code_id: self.token_id.unwrap(),
            owner: owner.to_string(),
            name: "Staking token".to_string(),
            symbol: "stake".to_string(),
            decimals: 6,
            epoch_period: 259200,   // 3 * 24 * 60 * 60 = 3 days
            unbond_period: 1814400, // 21 * 24 * 60 * 60 = 21 days
            validators: vec![
                "val1".to_string(),
                "val2".to_string(),
                "val3".to_string(),
                "val4".to_string(),
            ],
            protocol_fee_contract: "fee".to_string(),
            protocol_reward_fee: Decimal::from_ratio(1u128, 100u128),
        };

        let instance =
            router.instantiate_contract(code_id, owner, &init_msg, &[], "Hub", None).unwrap();

        self.hub = Some(ContractInfo {
            address: instance,
            code_id,
        })
        .into()
    }

    fn init_voting_escrow(&mut self, router: &mut App, owner: Addr) {
        let voting_contract = Box::new(ContractWrapper::new_with_empty(
            voting_escrow::contract::execute,
            voting_escrow::contract::instantiate,
            voting_escrow::contract::query,
        ));

        let voting_code_id = router.store_code(voting_contract);

        let msg = eris::voting_escrow::InstantiateMsg {
            guardian_addr: Some("guardian".to_string()),
            marketing: None,
            owner: owner.to_string(),
            deposit_token_addr: self.amp_lp.get_address_string(),
            logo_urls_whitelist: vec![],
        };

        let voting_instance = router
            .instantiate_contract(voting_code_id, owner, &msg, &[], String::from("vxASTRO"), None)
            .unwrap();

        self.voting_escrow = Some(ContractInfo {
            address: voting_instance,
            code_id: voting_code_id,
        })
        .into()
    }

    fn init_emp_registry(&mut self, router: &mut App, owner: Addr) {
        let contract = Box::new(ContractWrapper::new_with_empty(
            emp_gauges::contract::execute,
            emp_gauges::contract::instantiate,
            emp_gauges::contract::query,
        ));

        let code_id = router.store_code(contract);

        let msg = eris::emp_gauges::InstantiateMsg {
            owner: owner.to_string(),
            hub_addr: self.hub.get_address_string(),
            validators_limit: 30,
        };

        let instance = router
            .instantiate_contract(code_id, owner, &msg, &[], String::from("vxASTRO"), None)
            .unwrap();

        self.emp_gauges = Some(ContractInfo {
            address: instance,
            code_id,
        })
        .into()
    }

    fn init_amp_gauges(&mut self, router: &mut App, owner: Addr) {
        let contract = Box::new(ContractWrapper::new_with_empty(
            amp_gauges::contract::execute,
            amp_gauges::contract::instantiate,
            amp_gauges::contract::query,
        ));

        let code_id = router.store_code(contract);

        let msg = eris::amp_gauges::InstantiateMsg {
            owner: owner.to_string(),
            hub_addr: self.hub.get_address_string(),
            escrow_addr: self.voting_escrow.get_address_string(),
            emp_registry_addr: self.emp_gauges.get_address_string(),
            validators_limit: 30,
        };

        let instance = router
            .instantiate_contract(code_id, owner, &msg, &[], String::from("vxASTRO"), None)
            .unwrap();

        self.amp_gauges = Some(ContractInfo {
            address: instance,
            code_id,
        })
        .into()
    }

    fn init_hub_delegation_strategy(&mut self, router: &mut App, owner: Addr) {
        router
            .execute_contract(
                owner.clone(),
                self.hub.get_address(),
                &eris::hub::ExecuteMsg::UpdateConfig {
                    protocol_fee_contract: None,
                    protocol_reward_fee: None,
                    delegation_strategy: Some(eris::hub::DelegationStrategy::Gauges {
                        amp_gauges: self.amp_gauges.get_address_string(),
                        emp_gauges: self.emp_gauges.get_address_string(),
                        amp_factor_bps: 5000,
                        min_delegation_bps: 100,
                        max_delegation_bps: 2500,
                        validator_count: 5,
                    }),
                },
                &[],
            )
            .unwrap();

        router
            .execute_contract(
                owner,
                self.voting_escrow.get_address(),
                &eris::voting_escrow::ExecuteMsg::UpdateConfig {
                    new_guardian: None,
                    push_update_contracts: Some(vec![self.amp_gauges.get_address_string()]),
                },
                &[],
            )
            .unwrap();
    }

    // pub fn create_lock(
    //     &self,
    //     router: &mut App,
    //     user: Addr,
    //     time: u64,
    //     amount: u64,
    // ) -> Result<AppResponse> {
    //     let amount = amount * MULTIPLIER;
    //     let cw20msg = Cw20ExecuteMsg::Send {
    //         contract: self.voting_escrow.clone().unwrap().address.to_string(),
    //         amount: Uint128::from(amount),
    //         msg: to_binary(&Cw20HookMsg::CreateLock {
    //             time,
    //         })
    //         .unwrap(),
    //     };

    //     router.execute_contract(user, self.get_staking_xastro(router), &cw20msg, &[])
    // }

    // pub fn extend_lock_amount(
    //     &mut self,
    //     router: &mut App,
    //     user: &str,
    //     amount: u64,
    // ) -> Result<AppResponse> {
    //     let amount = amount * MULTIPLIER;
    //     let cw20msg = Cw20ExecuteMsg::Send {
    //         contract: self.voting_escrow.clone().unwrap().address.to_string(),
    //         amount: Uint128::from(amount),
    //         msg: to_binary(&Cw20HookMsg::ExtendLockAmount {}).unwrap(),
    //     };
    //     router.execute_contract(
    //         Addr::unchecked(user),
    //         self.get_staking_xastro(router),
    //         &cw20msg,
    //         &[],
    //     )
    // }

    // pub fn extend_lock_time(
    //     &mut self,
    //     router: &mut App,
    //     user: &str,
    //     time: u64,
    // ) -> Result<AppResponse> {
    //     router.execute_contract(
    //         Addr::unchecked(user),
    //         self.voting_escrow.clone().unwrap().address,
    //         &ExecuteMsg::ExtendLockTime {
    //             time,
    //         },
    //         &[],
    //     )
    // }

    // pub fn withdraw(&self, router: &mut App, user: &str) -> Result<AppResponse> {
    //     router.execute_contract(
    //         Addr::unchecked(user),
    //         self.voting_escrow.clone().unwrap().address,
    //         &ExecuteMsg::Withdraw {},
    //         &[],
    //     )
    // }

    // pub fn query_user_vp(&self, router: &mut App, user: Addr) -> StdResult<f32> {
    //     router
    //         .wrap()
    //         .query_wasm_smart(
    //             self.voting_escrow.clone().unwrap().address,
    //             &QueryMsg::UserVotingPower {
    //                 user: user.to_string(),
    //             },
    //         )
    //         .map(|vp: VotingPowerResponse| vp.voting_power.u128() as f32 / MULTIPLIER as f32)
    // }

    // pub fn query_user_vp_at(&self, router: &mut App, user: Addr, time: u64) -> StdResult<f32> {
    //     router
    //         .wrap()
    //         .query_wasm_smart(
    //             self.voting_escrow.clone().unwrap().address,
    //             &QueryMsg::UserVotingPowerAt {
    //                 user: user.to_string(),
    //                 time,
    //             },
    //         )
    //         .map(|vp: VotingPowerResponse| vp.voting_power.u128() as f32 / MULTIPLIER as f32)
    // }

    // pub fn query_total_vp(&self, router: &mut App) -> StdResult<f32> {
    //     router
    //         .wrap()
    //         .query_wasm_smart(
    //             self.voting_escrow.clone().unwrap().address,
    //             &QueryMsg::TotalVotingPower {},
    //         )
    //         .map(|vp: VotingPowerResponse| vp.voting_power.u128() as f32 / MULTIPLIER as f32)
    // }

    // pub fn query_total_vp_at(&self, router: &mut App, time: u64) -> StdResult<f32> {
    //     router
    //         .wrap()
    //         .query_wasm_smart(
    //             self.voting_escrow.clone().unwrap().address,
    //             &QueryMsg::TotalVotingPowerAt {
    //                 time,
    //             },
    //         )
    //         .map(|vp: VotingPowerResponse| vp.voting_power.u128() as f32 / MULTIPLIER as f32)
    // }
}

pub fn mint(router: &mut App, owner: Addr, token_instance: Addr, to: &Addr, amount: u128) {
    let amount = amount * MULTIPLIER as u128;
    let msg = cw20::Cw20ExecuteMsg::Mint {
        recipient: to.to_string(),
        amount: Uint128::from(amount),
    };

    let res = router.execute_contract(owner, token_instance, &msg, &[]).unwrap();
    assert_eq!(res.events[1].attributes[1], attr("action", "mint"));
    assert_eq!(res.events[1].attributes[2], attr("to", String::from(to)));
    assert_eq!(res.events[1].attributes[3], attr("amount", Uint128::from(amount)));
}

pub fn check_balance(app: &mut App, token_addr: &Addr, contract_addr: &Addr, expected: u128) {
    let msg = Cw20QueryMsg::Balance {
        address: contract_addr.to_string(),
    };
    let res: StdResult<BalanceResponse> = app.wrap().query_wasm_smart(token_addr, &msg);
    assert_eq!(res.unwrap().balance, Uint128::from(expected));
}

pub fn increase_allowance(
    router: &mut App,
    owner: Addr,
    spender: Addr,
    token: Addr,
    amount: Uint128,
) {
    let msg = cw20::Cw20ExecuteMsg::IncreaseAllowance {
        spender: spender.to_string(),
        amount,
        expires: None,
    };

    let res = router.execute_contract(owner.clone(), token, &msg, &[]).unwrap();

    assert_eq!(res.events[1].attributes[1], attr("action", "increase_allowance"));
    assert_eq!(res.events[1].attributes[2], attr("owner", owner.to_string()));
    assert_eq!(res.events[1].attributes[3], attr("spender", spender.to_string()));
    assert_eq!(res.events[1].attributes[4], attr("amount", amount));
}
