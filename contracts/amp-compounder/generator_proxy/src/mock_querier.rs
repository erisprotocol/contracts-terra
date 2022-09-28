use std::collections::HashMap;
use std::ops::Deref;
use cosmwasm_std::{Addr, BalanceResponse, BankQuery, Binary, Coin, ContractResult, Empty, from_binary, from_slice, OwnedDeps, Querier, QuerierResult, QueryRequest, StdResult, SystemError, SystemResult, to_binary, Uint128, WasmQuery};
use cosmwasm_std::testing::{MockApi, MockStorage};
use cw_storage_plus::Map;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use astroport::asset::{AssetInfo, token_asset, token_asset_info};
use astroport::generator::{PendingTokenResponse, UserInfoV2};
use astroport_governance::voting_escrow::{LockInfoResponse, VotingPowerResponse};
use crate::astro_gov::Lock;

pub fn mock_dependencies() -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier: WasmMockQuerier = WasmMockQuerier::new();

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: Default::default()
    }
}

const ASTRO_TOKEN: &str = "astro";
const REWARD_TOKEN: &str = "reward";
const GENERATOR: &str = "generator";
const USER_INFO: Map<(&Addr, &Addr), UserInfoV2> = Map::new("user_info");
const PROXY_REWARD_ASSET: Map<&Addr, AssetInfo> = Map::new("proxy_reward_asset");
const LOCK: Map<Addr, Lock> = Map::new("locked");
const REWARDS_PER_WEEK: Map<u64, Uint128> = Map::new("rewards_per_week");
const LAST_CLAIM_PERIOD: Map<Addr, u64> = Map::new("last_claim_period");

const VOTING_ESCROW: &str = "voting_escrow";
const FEE_DISTRIBUTOR: &str = "fee_distributor";

pub struct WasmMockQuerier {
    balances: HashMap<(String, String), Uint128>,
    raw: HashMap<(String, Binary), Binary>,
}

impl WasmMockQuerier {
    pub fn new() -> Self {
        WasmMockQuerier {
            balances: HashMap::new(),
            raw: HashMap::new(),
        }
    }

    pub fn set_balance(&mut self, token: String, addr: String, amount: Uint128) {
        self.balances.insert((token, addr), amount);
    }

    pub fn set_user_info(&mut self, lp_token: &Addr, user: &Addr, user_info: &UserInfoV2) -> StdResult<()> {
        let key = Binary::from(USER_INFO.key((lp_token, user)).deref());
        self.raw.insert((GENERATOR.to_string(), key), to_binary(user_info)?);

        Ok(())
    }

    pub fn set_reward_proxy(&mut self, proxy_addr: &Addr, token: &Addr) -> StdResult<()> {
        let key = Binary::from(PROXY_REWARD_ASSET.key(proxy_addr).deref());
        self.raw.insert((GENERATOR.to_string(), key), to_binary(&token_asset_info(token.clone()))?);

        Ok(())
    }

    pub fn set_lock(&mut self, user: Addr, lock: &Lock) -> StdResult<()> {
        let key = Binary::from(LOCK.key(user).deref());
        self.raw.insert((VOTING_ESCROW.to_string(), key), to_binary(lock)?);

        Ok(())
    }

    pub fn set_last_claim_period(&mut self, user: Addr, period: u64) -> StdResult<()> {
        let key = Binary::from(LAST_CLAIM_PERIOD.key(user).deref());
        self.raw.insert((FEE_DISTRIBUTOR.to_string(), key), to_binary(&period)?);

        Ok(())
    }

    pub fn set_rewards_per_week(&mut self, period: u64, amount: Uint128) -> StdResult<()> {
        let key = Binary::from(REWARDS_PER_WEEK.key(period).deref());
        self.raw.insert((FEE_DISTRIBUTOR.to_string(), key), to_binary(&amount)?);

        Ok(())
    }

    fn get_balance(&self, token: String, addr: String) -> Uint128 {
        *self.balances.get(&(token, addr)).unwrap_or(&Uint128::zero())
    }

    fn execute_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        let result = match request {
            QueryRequest::Bank(BankQuery::Balance {
                                   address,
                                   denom,
                               }) => {
                let amount = self.get_balance(denom.clone(), address.clone());
                to_binary(&BalanceResponse {
                    amount: Coin {
                        denom: denom.clone(),
                        amount,
                    },
                })
            },
            QueryRequest::Wasm(WasmQuery::Smart {
                                   contract_addr,
                                   msg,
                               }) => self.execute_wasm_query(contract_addr, msg),
            QueryRequest::Wasm(WasmQuery::Raw {
                                   contract_addr,
                                   key,
                               }) => {
                let value = self.raw.get(&(contract_addr.clone(), key.clone()));
                if let Some(binary) = value {
                    Ok(binary.clone())
                } else {
                    Ok(Binary::default())
                }
            },
            _ => return QuerierResult::Err(SystemError::Unknown {}),
        };
        QuerierResult::Ok(ContractResult::from(result))
    }

    fn execute_wasm_query(&self, contract_addr: &String, msg: &Binary) -> StdResult<Binary> {
        match from_binary(msg)? {
            MockQueryMsg::Balance {
                address,
            } => {
                let balance = self.get_balance(contract_addr.clone(), address);
                to_binary(&cw20::BalanceResponse {
                    balance,
                })
            },
            MockQueryMsg::Deposit {
                lp_token,
                ..
            } => {
                let balance = self.get_balance(contract_addr.clone(), lp_token);
                to_binary(&balance)
            },
            MockQueryMsg::PendingToken { .. } => {
                let pending = self.get_balance(contract_addr.clone(), ASTRO_TOKEN.to_string());
                let reward = self.get_balance(contract_addr.clone(), REWARD_TOKEN.to_string());
                to_binary(&PendingTokenResponse {
                    pending,
                    pending_on_proxy: Some(vec![
                        token_asset(Addr::unchecked(REWARD_TOKEN), reward),
                    ]),
                })
            },
            MockQueryMsg::LockInfo { user } => {
                let key = Binary::from(LOCK.key(Addr::unchecked(user)).deref());
                let value = self.raw.get(&(contract_addr.clone(), key));
                let lock: Lock = if let Some(value) = value {
                    from_binary(value)?
                } else {
                    Lock::default()
                };
                to_binary(&LockInfoResponse {
                    amount: lock.amount,
                    coefficient: Default::default(),
                    start: lock.start,
                    end: lock.end,
                    slope: Default::default()
                })
            },
            MockQueryMsg::UserVotingPowerAtPeriod { user, .. } => {
                let voting_power = self.get_balance(contract_addr.clone(), user);
                to_binary(&VotingPowerResponse { voting_power })
            },
            MockQueryMsg::TotalVotingPowerAtPeriod { .. } => {
                let voting_power = self.get_balance(contract_addr.clone(), contract_addr.clone());
                to_binary(&VotingPowerResponse { voting_power })
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum MockQueryMsg {
    Balance {
        address: String,
    },
    Deposit {
        lp_token: String,
        user: String,
    },
    PendingToken {
        lp_token: String,
        user: String
    },
    LockInfo { user: String },
    UserVotingPowerAtPeriod { user: String, period: u64 },
    TotalVotingPowerAtPeriod { period: u64 },
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<Empty> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.execute_query(&request)
    }
}
