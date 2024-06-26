use cosmwasm_std::{Addr, BankMsg, Coin, CosmosMsg, StakingMsg};

use crate::constants::CONTRACT_DENOM;

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct Delegation {
    pub validator: String,
    pub amount: u128,
}

impl Delegation {
    pub fn new(validator: &str, amount: u128) -> Self {
        Self {
            validator: validator.to_string(),
            amount,
        }
    }

    pub fn to_cosmos_msg(&self) -> CosmosMsg {
        CosmosMsg::Staking(StakingMsg::Delegate {
            validator: self.validator.clone(),
            amount: Coin::new(self.amount, CONTRACT_DENOM),
        })
    }
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct SendFee {
    pub to_address: String,
    pub amount: u128,
}

impl SendFee {
    pub fn new(to_address: Addr, amount: u128) -> Self {
        Self {
            to_address: to_address.to_string(),
            amount,
        }
    }

    pub fn to_cosmos_msg(&self) -> CosmosMsg {
        CosmosMsg::Bank(BankMsg::Send {
            to_address: self.to_address.clone(),
            amount: vec![Coin::new(self.amount, CONTRACT_DENOM)],
        })
    }
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct Undelegation {
    pub validator: String,
    pub amount: u128,
}

impl Undelegation {
    pub fn new(validator: &str, amount: u128) -> Self {
        Self {
            validator: validator.to_string(),
            amount,
        }
    }

    pub fn to_cosmos_msg(&self) -> CosmosMsg {
        CosmosMsg::Staking(StakingMsg::Undelegate {
            validator: self.validator.clone(),
            amount: Coin::new(self.amount, CONTRACT_DENOM),
        })
    }
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct Redelegation {
    pub src: String,
    pub dst: String,
    pub amount: u128,
}

impl Redelegation {
    pub fn new(src: &str, dst: &str, amount: u128) -> Self {
        Self {
            src: src.to_string(),
            dst: dst.to_string(),
            amount,
        }
    }

    pub fn to_cosmos_msg(&self) -> CosmosMsg {
        CosmosMsg::Staking(StakingMsg::Redelegate {
            src_validator: self.src.clone(),
            dst_validator: self.dst.clone(),
            amount: Coin::new(self.amount, CONTRACT_DENOM),
        })
    }
}
