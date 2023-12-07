use astroport::asset::AssetInfo;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, DepsMut, StdError, StdResult, Uint128, WasmMsg,
};

type IncentiveResponse = Option<Addr>;

#[cw_serde]
pub enum QueryMsg {
    Incentive {
        lp_asset: AssetInfo,
    },
    Positions {
        address: String,
    },
}

#[cw_serde]
pub struct PositionsResponse {
    /// The current time of the blockchain.
    pub timestamp: u64,
    /// All the positions a user has.
    pub positions: Vec<QueryPosition>,
}

#[cw_serde]
pub enum QueryPosition {
    /// Represents a position that a user has deposited, but not yet begun to unbond.
    OpenPosition {
        /// The amount of LP tokens the user deposited into the position.
        amount: Uint128,
        /// The amount of time (in seconds) the user must wait after they begin the unbonding process.
        unbonding_duration: u64,
        /// The amount of weight the position has.
        weight: Uint128,
    },
    /// Represents a position that a user has initiated the unbonding process on. The position may or may not be withdrawable.
    ClosedPosition {
        /// The amount of LP tokens the user deposited into the position, and will receive after they withdraw.
        amount: Uint128,
        /// The timestamp (in seconds) the user unbonded at.
        unbonding_timestamp: u64,
        /// The amount of weight the position has.
        weight: Uint128,
    },
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Creates a new position to earn flow rewards.
    OpenPosition {
        /// The amount to add to the position.
        amount: Uint128,
        /// The amount of time (in seconds) before the LP tokens can be redeemed.
        unbonding_duration: u64,
        /// The receiver of the new position.
        ///
        /// This is mostly used for the frontend helper contract.
        ///
        /// If left empty, defaults to the message sender.
        receiver: Option<String>,
    },
    /// Expands an existing position to earn more flow rewards.
    ExpandPosition {
        /// The amount to add to the existing position.
        amount: Uint128,
        /// The unbond completion timestamp to identify the position to add to.
        unbonding_duration: u64,
        /// The receiver of the expanded position.
        ///
        /// This is mostly used for the frontend helper contract.
        ///
        /// If left empty, defaults to the message sender.
        receiver: Option<String>,
    },
}

pub fn get_incentive_contract(
    deps: &DepsMut,
    incentive_factory_addr: String,
    lp: &AssetInfo,
) -> StdResult<Addr> {
    let incentive_address: IncentiveResponse = deps.querier.query_wasm_smart(
        incentive_factory_addr,
        &QueryMsg::Incentive {
            lp_asset: lp.clone(),
        },
    )?;

    if let Some(incentive_address) = incentive_address {
        Ok(incentive_address)
    } else {
        Err(StdError::NotFound {
            kind: "incentive contract".to_string(),
        })
    }
}

pub fn get_open_or_extend_lock(
    deps: &DepsMut,
    incentive_address: Addr,
    unbonding_duration: u64,
    lp_amount: Uint128,
    user: &Addr,
    funds: Vec<Coin>,
) -> StdResult<CosmosMsg> {
    let positions: PositionsResponse = deps.querier.query_wasm_smart(
        incentive_address.clone(),
        &QueryMsg::Positions {
            address: user.to_string(),
        },
    )?;
    let has_existing_position = positions.positions.into_iter().any(|position| match position {
        QueryPosition::OpenPosition {
            unbonding_duration: position_unbonding_duration,
            ..
        } => unbonding_duration == position_unbonding_duration,
        QueryPosition::ClosedPosition {
            ..
        } => false,
    });

    let execute = match has_existing_position {
        true => ExecuteMsg::ExpandPosition {
            amount: lp_amount,
            unbonding_duration,
            receiver: None,
        },
        false => ExecuteMsg::OpenPosition {
            amount: lp_amount,
            unbonding_duration,
            receiver: None,
        },
    };

    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: incentive_address.to_string(),
        msg: to_binary(&execute)?,
        funds,
    }))
}
