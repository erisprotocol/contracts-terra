use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint128};

#[cw_serde]
pub struct StaderConfigUpdateRequest {
    pub(crate) min_deposit: Option<Uint128>,
    pub(crate) max_deposit: Option<Uint128>,

    pub(crate) cw20_token_contract: Option<String>, // Only upgradeable once.
    pub(crate) protocol_reward_fee: Option<Decimal>,
    pub(crate) protocol_withdraw_fee: Option<Decimal>,
    pub(crate) protocol_deposit_fee: Option<Decimal>,
    pub(crate) airdrop_registry_contract: Option<String>,

    pub(crate) unbonding_period: Option<u64>,
    pub(crate) undelegation_cooldown: Option<u64>,
    pub(crate) reinvest_cooldown: Option<u64>,
}

#[cw_serde]
pub enum StaderExecuteMsg {
    UpdateConfig {
        config_request: StaderConfigUpdateRequest,
    },
}
