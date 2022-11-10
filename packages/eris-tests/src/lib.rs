pub mod base;
use std::str::FromStr;

use anyhow::{Error, Ok, Result};
use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use cosmwasm_std::{coin, Addr, Attribute, BlockInfo, Decimal, Timestamp, Validator};
use cw_multi_test::{App, AppResponse, BankKeeper, BasicAppBuilder, StakeKeeper, StakingInfo};
use eris::governance_helper::{get_period, EPOCH_START, WEEK};

#[allow(clippy::all)]
#[allow(dead_code)]
pub mod gov_helper;

pub fn mock_app() -> App {
    mock_app_validators(None)
}

pub fn mock_app_validators(validators: Option<u64>) -> App {
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(EPOCH_START);
    let api = MockApi::default();
    let bank = BankKeeper::new();
    let storage = MockStorage::new();
    let staking = StakeKeeper::new();

    let block = BlockInfo {
        time: Timestamp::from_seconds(0),
        height: 0,
        chain_id: "".to_string(),
    };

    let validators = validators.unwrap_or(4);

    BasicAppBuilder::new()
        .with_api(api)
        .with_block(env.block)
        .with_bank(bank)
        .with_storage(storage)
        .with_staking(staking)
        .build(|router, api, storage| {
            router
                .bank
                .init_balance(storage, &Addr::unchecked("user1"), vec![coin(1000_000000, "uluna")])
                .unwrap();

            router
                .staking
                .setup(
                    storage,
                    StakingInfo {
                        bonded_denom: "uluna".to_string(),
                        apr: Decimal::from_str("0.13").unwrap(),
                        unbonding_time: 1814400,
                    },
                )
                .unwrap();

            for i in 1..validators {
                router
                    .staking
                    .add_validator(
                        api,
                        storage,
                        &block,
                        Validator {
                            address: format!("val{0}", i),
                            commission: Decimal::from_str("0.05").unwrap(),
                            max_commission: Decimal::from_str("0.05").unwrap(),
                            max_change_rate: Decimal::from_str("0.05").unwrap(),
                        },
                    )
                    .unwrap();
            }
        })
}

pub trait TerraAppExtension {
    fn next_block(&mut self, time: u64);
    fn next_period(&mut self, periods: u64);
    fn block_period(&self) -> u64;
}

impl TerraAppExtension for App {
    fn next_block(&mut self, time: u64) {
        self.update_block(|block| {
            block.time = block.time.plus_seconds(time);
            block.height += 1
        });
    }

    fn next_period(&mut self, periods: u64) {
        self.update_block(|block| {
            block.time = block.time.plus_seconds(periods * WEEK);
            block.height += 1000
        });
    }

    fn block_period(&self) -> u64 {
        get_period(self.block_info().time.seconds()).unwrap()
    }
}

pub trait EventChecker {
    fn assert_attribute(&self, ty: impl Into<String>, attr: Attribute) -> Result<()>;
}

impl EventChecker for AppResponse {
    fn assert_attribute(&self, ty: impl Into<String>, attr: Attribute) -> Result<()> {
        let ty: String = ty.into();
        let found = self.events.iter().any(|a| {
            a.ty == ty && a.attributes.iter().any(|b| b.key == attr.key && b.value == attr.value)
        });

        if !found {
            println!("{:?}", self.events);
            let text = format!("Could not find key: {0} value: {1}", attr.key, attr.value);
            // panic!("{}", text);
            return Err(Error::msg(text));
        }

        Ok(())
    }
}
