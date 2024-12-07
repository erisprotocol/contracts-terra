use cosmwasm_std::testing::{MockApi, MockStorage};
use cosmwasm_std::{
    from_binary, from_slice, to_json_binary, Addr, BalanceResponse, BankQuery, Binary, Coin,
    ContractResult, Empty, OwnedDeps, Querier, QuerierResult, QueryRequest, StdResult, SystemError,
    SystemResult, Uint128, WasmQuery,
};
use cw20::Cw20QueryMsg;
use eris::compound_proxy::LpStateResponse;
use std::collections::HashMap;
use std::vec;

use astroport::asset::{native_asset, token_asset};
use astroport::generator::PendingTokenResponse;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::cw20_querier::Cw20Querier;

pub fn mock_dependencies() -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier: WasmMockQuerier = WasmMockQuerier::new();

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: Default::default(),
    }
}

const ASTRO_TOKEN: &str = "astro";
const REWARD_TOKEN: &str = "reward";

pub struct WasmMockQuerier {
    cw20_querier: Cw20Querier,
    balances: HashMap<(String, String), Uint128>,
    raw: HashMap<(String, Binary), Binary>,
}

impl WasmMockQuerier {
    pub fn new() -> Self {
        WasmMockQuerier {
            cw20_querier: Cw20Querier::default(),
            balances: HashMap::new(),
            raw: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn set_cw20_balance(&mut self, token: &str, user: &str, balance: u128) {
        match self.cw20_querier.balances.get_mut(token) {
            Some(contract_balances) => {
                contract_balances.insert(user.to_string(), balance);
            },
            None => {
                let mut contract_balances: HashMap<String, u128> = HashMap::default();
                contract_balances.insert(user.to_string(), balance);
                self.cw20_querier.balances.insert(token.to_string(), contract_balances);
            },
        };
    }

    pub fn get_cw20_balance(&mut self, token: &str, user: &str) -> u128 {
        match self.cw20_querier.balances.get_mut(token) {
            Some(contract_balances) => *contract_balances.get(&user.to_string()).unwrap_or(&0u128),
            None => 0u128,
        }
    }

    pub fn set_cw20_total_supply(&mut self, token: &str, total_supply: u128) {
        self.cw20_querier.total_supplies.insert(token.to_string(), total_supply);
    }

    pub fn get_cw20_total_supply(&mut self, token: &str) -> u128 {
        *self.cw20_querier.total_supplies.get(&token.to_string()).unwrap_or(&0u128)
    }

    pub fn set_balance(&mut self, token: &str, addr: &str, amount: u128) {
        self.balances.insert((token.to_string(), addr.to_string()), amount.into());
    }

    pub fn set_generator_pending(&mut self, token: &str, addr: &str, amount: u128) {
        // generator pending tokens are stored reversed in the balances
        self.balances.insert((addr.to_string(), token.to_string()), amount.into());
    }

    pub fn get_balance(&self, token: String, addr: String) -> Uint128 {
        *self.balances.get(&(token, addr)).unwrap_or(&Uint128::zero())
    }

    fn execute_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        let result = match request {
            QueryRequest::Bank(BankQuery::Balance {
                address,
                denom,
            }) => {
                let amount = self.get_balance(denom.clone(), address.clone());
                to_json_binary(&BalanceResponse {
                    amount: Coin {
                        denom: denom.clone(),
                        amount,
                    },
                })
            },
            QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr,
                msg,
            }) => {
                if let Ok(query) = from_binary::<Cw20QueryMsg>(msg) {
                    return self.cw20_querier.handle_query(contract_addr, query);
                }

                self.execute_wasm_query(contract_addr, msg)
            },

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

    fn execute_wasm_query(&self, contract_addr: &str, msg: &Binary) -> StdResult<Binary> {
        match from_binary(msg)? {
            // MockQueryMsg::Balance {
            //     address,
            // } => {
            //     let balance = self.get_balance(contract_addr.clone(), address);
            //     to_json_binary(&cw20::BalanceResponse {
            //         balance,
            //     })
            // },
            MockQueryMsg::Deposit {
                lp_token,
                ..
            } => {
                let balance = self.get_balance(contract_addr.to_string(), lp_token);
                to_json_binary(&balance)
            },
            MockQueryMsg::PendingToken {
                ..
            } => {
                let pending = self.get_balance(contract_addr.to_string(), ASTRO_TOKEN.to_string());
                let reward = self.get_balance(contract_addr.to_string(), REWARD_TOKEN.to_string());
                to_json_binary(&PendingTokenResponse {
                    pending,
                    pending_on_proxy: Some(vec![token_asset(
                        Addr::unchecked(REWARD_TOKEN),
                        reward,
                    )]),
                })
            },
            MockQueryMsg::GetLpState {
                lp_addr,
            } => to_json_binary(&LpStateResponse {
                contract_addr: Addr::unchecked("pair"),
                liquidity_token: Addr::unchecked(lp_addr),
                assets: vec![
                    native_asset("asset1".to_string(), Uint128::new(1000000u128)),
                    token_asset(Addr::unchecked("asset2"), Uint128::new(2000000u128)),
                ],
                total_share: Uint128::new(10_000000u128),
            }),
        }
    }
}

impl Default for WasmMockQuerier {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum MockQueryMsg {
    // Balance {
    //     address: String,
    // },
    Deposit {
        lp_token: String,
        user: String,
    },
    PendingToken {
        lp_token: String,
        user: String,
    },
    GetLpState {
        lp_addr: String,
    },
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
            },
        };
        self.execute_query(&request)
    }
}
