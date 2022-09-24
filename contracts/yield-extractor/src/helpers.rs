use cosmwasm_std::{
    Addr, Decimal, QuerierWrapper, Reply, StdError, StdResult, SubMsgResponse, Uint128,
};
use cw20::{BalanceResponse, Cw20QueryMsg, TokenInfoResponse};
use eris_staking::{
    hub::{StaderStateResponse, StateResponse, SteakStateResponse},
    yieldextractor::LiquidStakingType,
};

/// Unwrap a `Reply` object to extract the response
pub(crate) fn unwrap_reply(reply: Reply) -> StdResult<SubMsgResponse> {
    reply.result.into_result().map_err(StdError::generic_err)
}

/// Query the total supply of a CW20 token
pub(crate) fn query_cw20_total_supply(
    querier: &QuerierWrapper,
    token_addr: &Addr,
) -> StdResult<Uint128> {
    let token_info: TokenInfoResponse =
        querier.query_wasm_smart(token_addr, &Cw20QueryMsg::TokenInfo {})?;
    Ok(token_info.total_supply)
}

/// Query the total supply of a CW20 token
pub(crate) fn query_cw20_balance(
    querier: &QuerierWrapper,
    token_addr: &Addr,
    address: &Addr,
) -> StdResult<Uint128> {
    let balance_response: BalanceResponse = querier.query_wasm_smart(
        token_addr,
        &Cw20QueryMsg::Balance {
            address: address.to_string(),
        },
    )?;
    Ok(balance_response.balance)
}

/// Query the total supply of a CW20 token
pub(crate) fn query_exchange_rate(
    querier: &QuerierWrapper,
    interface: LiquidStakingType,
    hub_addr: &Addr,
) -> StdResult<Decimal> {
    match interface {
        LiquidStakingType::Eris => {
            let response: StateResponse =
                querier.query_wasm_smart(hub_addr, &eris_staking::hub::QueryMsg::State {})?;

            Ok(response.exchange_rate)
        },
        LiquidStakingType::Stader => {
            let response: StaderStateResponse =
                querier.query_wasm_smart(hub_addr, &eris_staking::hub::QueryMsg::State {})?;

            Ok(response.state.exchange_rate)
        },
        LiquidStakingType::Steak => {
            let response: SteakStateResponse =
                querier.query_wasm_smart(hub_addr, &eris_staking::hub::QueryMsg::State {})?;

            Ok(response.exchange_rate)
        },
    }
}
