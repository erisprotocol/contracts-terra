use std::cmp::Ordering;

use crate::{
    contract::calculate_optimal_swap,
    state::{Config, CONFIG, PAIR_PROXY},
};
use astroport::{asset::AssetInfo, pair::StablePoolConfig, querier::query_token_precision, U256};

use astroport::querier::query_supply;
use cosmwasm_std::{from_binary, CosmosMsg, Deps, Fraction, StdError, StdResult, Uint128};

use eris::compound_proxy::CompoundSimulationResponse;

use astroport::asset::{Asset, AssetInfoExt};
use astroport::factory::PairType;
use eris::adapters::pair::Pair;

const ITERATIONS: u8 = 32;

const N_COINS: u8 = 2;
const AMP_PRECISION: u64 = 100;

/// ## Description
/// Returns simulated amount of LP token from given rewards in a [`CompoundSimulationResponse`].
pub fn query_compound_simulation(
    deps: Deps,
    rewards: Vec<Asset>,
) -> StdResult<CompoundSimulationResponse> {
    let config: Config = CONFIG.load(deps.storage)?;
    let asset_a_info = config.pair_info.asset_infos[0].clone();
    let asset_b_info = config.pair_info.asset_infos[1].clone();
    let mut asset_a_amount = Uint128::zero();
    let mut asset_b_amount = Uint128::zero();

    for reward in rewards {
        let pair_proxy = PAIR_PROXY.may_load(deps.storage, reward.info.to_string())?;
        let add_asset = if let Some(pair_proxy) = pair_proxy {
            let simulation_response = pair_proxy.simulate(&deps.querier, &reward, None)?;
            let pair_proxy_info = pair_proxy.query_pair_info(&deps.querier)?;
            let return_asset_info = if reward.info.equal(&pair_proxy_info.asset_infos[0]) {
                &pair_proxy_info.asset_infos[1]
            } else if reward.info.equal(&pair_proxy_info.asset_infos[1]) {
                &pair_proxy_info.asset_infos[0]
            } else {
                return Err(StdError::generic_err("Invalid pair proxy"));
            };
            return_asset_info.with_balance(simulation_response.return_amount)
        } else {
            reward
        };
        if add_asset.info.equal(&asset_a_info) {
            asset_a_amount += add_asset.amount;
        } else if add_asset.info.equal(&asset_b_info) {
            asset_b_amount += add_asset.amount;
        } else {
            return Err(StdError::generic_err("Invalid reward"));
        }
    }

    let pair = Pair(config.pair_info.contract_addr.clone());
    let mut pools = config.pair_info.query_pools(&deps.querier, &config.pair_info.contract_addr)?;

    let total_share = query_supply(&deps.querier, &config.pair_info.liquidity_token)?;

    let (lp_amount, swap_asset_a_amount, swap_asset_b_amount, return_a_amount, return_b_amount) =
        match config.pair_info.pair_type {
            PairType::Xyk {} => {
                let asset_a = Asset {
                    info: asset_a_info,
                    amount: asset_a_amount,
                };
                let asset_b = Asset {
                    info: asset_b_info,
                    amount: asset_b_amount,
                };
                let mut _messages: Vec<CosmosMsg> = vec![];
                let (swap_asset_a_amount, swap_asset_b_amount, return_a_amount, return_b_amount) =
                    calculate_optimal_swap(
                        &deps.querier,
                        &config,
                        asset_a,
                        asset_b,
                        None,
                        None,
                        &mut _messages,
                    )?;

                if !swap_asset_a_amount.is_zero() {
                    asset_a_amount -= swap_asset_a_amount;
                    asset_b_amount += return_b_amount;
                    pools[0].amount += swap_asset_a_amount;
                    pools[1].amount -= return_b_amount;
                }

                if !swap_asset_b_amount.is_zero() {
                    asset_b_amount -= swap_asset_b_amount;
                    asset_a_amount += return_a_amount;
                    pools[1].amount += swap_asset_b_amount;
                    pools[0].amount -= return_a_amount;
                }

                let lp_amount = if total_share.is_zero() {
                    Uint128::new(
                        (U256::from(asset_a_amount.u128()) * U256::from(asset_b_amount.u128()))
                            .integer_sqrt()
                            .as_u128(),
                    )
                } else {
                    std::cmp::min(
                        asset_a_amount.multiply_ratio(total_share, pools[0].amount),
                        asset_b_amount.multiply_ratio(total_share, pools[1].amount),
                    )
                };
                (
                    lp_amount,
                    swap_asset_a_amount,
                    swap_asset_b_amount,
                    return_a_amount,
                    return_b_amount,
                )
            },
            PairType::Stable {} => {
                let token_precision_0 = query_token_precision(&deps.querier, &asset_a_info)?;
                let token_precision_1 = query_token_precision(&deps.querier, &asset_b_info)?;

                let greater_precision = token_precision_0.max(token_precision_1);

                let deposit_amount_0 =
                    adjust_precision(asset_a_amount, token_precision_0, greater_precision)?;
                let deposit_amount_1 =
                    adjust_precision(asset_b_amount, token_precision_1, greater_precision)?;

                let lp_amount = if total_share.is_zero() {
                    let liquidity_token_precision = query_token_precision(
                        &deps.querier,
                        &AssetInfo::Token {
                            contract_addr: config.pair_info.liquidity_token,
                        },
                    )?;

                    // Initial share = collateral amount
                    adjust_precision(
                        Uint128::new(
                            (U256::from(deposit_amount_0.u128())
                                * U256::from(deposit_amount_1.u128()))
                            .integer_sqrt()
                            .as_u128(),
                        ),
                        greater_precision,
                        liquidity_token_precision,
                    )?
                } else {
                    let leverage = if let Some(params) = pair.query_config(&deps.querier)?.params {
                        let stable_pool_config: StablePoolConfig = from_binary(&params)?;
                        let amp = stable_pool_config.amp.numerator() * Uint128::from(AMP_PRECISION)
                            / stable_pool_config.amp.denominator();
                        u64::try_from(amp.u128()).unwrap_or(25u64)
                    } else {
                        25u64
                    };

                    let mut pool_amount_0 =
                        adjust_precision(pools[0].amount, token_precision_0, greater_precision)?;
                    let mut pool_amount_1 =
                        adjust_precision(pools[1].amount, token_precision_1, greater_precision)?;

                    let d_before_addition_liquidity =
                        compute_d(leverage, pool_amount_0.u128(), pool_amount_1.u128()).unwrap();

                    pool_amount_0 = pool_amount_0.checked_add(deposit_amount_0)?;
                    pool_amount_1 = pool_amount_1.checked_add(deposit_amount_1)?;

                    let d_after_addition_liquidity =
                        compute_d(leverage, pool_amount_0.u128(), pool_amount_1.u128()).unwrap();

                    // d after adding liquidity may be less than or equal to d before adding liquidity because of rounding
                    if d_before_addition_liquidity >= d_after_addition_liquidity {
                        Uint128::zero()
                    } else {
                        total_share.multiply_ratio(
                            d_after_addition_liquidity - d_before_addition_liquidity,
                            d_before_addition_liquidity,
                        )
                    }
                };

                (lp_amount, Uint128::zero(), Uint128::zero(), Uint128::zero(), Uint128::zero())
            },
            PairType::Custom(_) => {
                return Err(StdError::generic_err("Custom pair type not supported"));
            },
        };

    Ok(CompoundSimulationResponse {
        lp_amount,
        swap_asset_a_amount,
        swap_asset_b_amount,
        return_a_amount,
        return_b_amount,
    })
}

/// ## Description
/// Return a value using a newly specified precision.
/// ## Params
/// * **value** is an object of type [`Uint128`]. This is the value that will have its precision adjusted.
///
/// * **current_precision** is an object of type [`u8`]. This is the `value`'s current precision
///
/// * **new_precision** is an object of type [`u8`]. This is the new precision to use when returning the `value`.
fn adjust_precision(
    value: Uint128,
    current_precision: u8,
    new_precision: u8,
) -> StdResult<Uint128> {
    Ok(match current_precision.cmp(&new_precision) {
        Ordering::Equal => value,
        Ordering::Less => value
            .checked_mul(Uint128::new(10_u128.pow((new_precision - current_precision) as u32)))?,
        Ordering::Greater => value
            .checked_div(Uint128::new(10_u128.pow((current_precision - new_precision) as u32)))?,
    })
}

/// ## Description
/// Computes the stableswap invariant (D).
///
/// * **Equation**
///
/// A * sum(x_i) * n**n + D = A * D * n**n + D**(n+1) / (n**n * prod(x_i))
pub fn compute_d(leverage: u64, amount_a: u128, amount_b: u128) -> Option<u128> {
    let amount_a_times_coins =
        checked_u8_mul(&U256::from(amount_a), N_COINS)?.checked_add(U256::one())?;
    let amount_b_times_coins =
        checked_u8_mul(&U256::from(amount_b), N_COINS)?.checked_add(U256::one())?;
    let sum_x = amount_a.checked_add(amount_b)?; // sum(x_i), a.k.a S
    if sum_x == 0 {
        Some(0)
    } else {
        let mut d_previous: U256;
        let mut d: U256 = sum_x.into();

        // Newton's method to approximate D
        for _ in 0..ITERATIONS {
            let mut d_product = d;
            d_product = d_product.checked_mul(d)?.checked_div(amount_a_times_coins)?;
            d_product = d_product.checked_mul(d)?.checked_div(amount_b_times_coins)?;
            d_previous = d;
            // d = (leverage * sum_x + d_p * n_coins) * d / ((leverage - 1) * d + (n_coins + 1) * d_p);
            d = calculate_step(&d, leverage, sum_x, &d_product)?;
            // Equality with the precision of 1
            if d == d_previous {
                break;
            }
        }
        u128::try_from(d).ok()
    }
}

/// ## Description
/// Helper function used to calculate the D invariant as a last step in the `compute_d` public function.
///
/// * **Equation**:
///
/// d = (leverage * sum_x + d_product * n_coins) * initial_d / ((leverage - 1) * initial_d + (n_coins + 1) * d_product)
fn calculate_step(initial_d: &U256, leverage: u64, sum_x: u128, d_product: &U256) -> Option<U256> {
    let leverage_mul = U256::from(leverage).checked_mul(sum_x.into())? / AMP_PRECISION;
    let d_p_mul = checked_u8_mul(d_product, N_COINS)?;

    let l_val = leverage_mul.checked_add(d_p_mul)?.checked_mul(*initial_d)?;

    let leverage_sub =
        initial_d.checked_mul((leverage.checked_sub(AMP_PRECISION)?).into())? / AMP_PRECISION;
    let n_coins_sum = checked_u8_mul(d_product, N_COINS.checked_add(1)?)?;

    let r_val = leverage_sub.checked_add(n_coins_sum)?;

    l_val.checked_div(r_val)
}

/// Returns self multiplied by b.
fn checked_u8_mul(a: &U256, b: u8) -> Option<U256> {
    let mut result = *a;
    for _ in 1..b {
        result = result.checked_add(*a)?;
    }
    Some(result)
}
