/* tslint:disable */
/**
 * This file was automatically generated by json-schema-to-typescript.
 * DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
 * and run json-schema-to-typescript to regenerate this file.
 */

/**
 * A fixed-point decimal value with 18 fractional digits, i.e. Decimal(1_000_000_000_000_000_000) == 1.0
 *
 * The greatest possible value that can be represented is 340282366920938463463.374607431768211455 (which is (2^128 - 1) / 10^18)
 */
export type Decimal = string;
/**
 * A thin wrapper around u128 that is using strings for JSON encoding/decoding, such that the full u128 range can be used for clients that convert JSON numbers to floats, like JavaScript and jq.
 *
 * # Examples
 *
 * Use `from` to create instances of this and `u128` to get the value out:
 *
 * ``` # use cosmwasm_std::Uint128; let a = Uint128::from(123u128); assert_eq!(a.u128(), 123);
 *
 * let b = Uint128::from(42u64); assert_eq!(b.u128(), 42);
 *
 * let c = Uint128::from(70u32); assert_eq!(c.u128(), 70); ```
 */
export type Uint128 = string;

export interface StateResponse {
  /**
   * The exchange rate between ustake and uluna, in terms of uluna per ustake
   */
  exchange_rate: Decimal;
  /**
   * Total amount of uluna staked
   */
  total_uluna: Uint128;
  /**
   * Total supply to the Stake token
   */
  total_ustake: Uint128;
  /**
   * Staking rewards currently held by the contract that are ready to be reinvested
   */
  unlocked_coins: Coin[];
  [k: string]: unknown;
}
export interface Coin {
  amount: Uint128;
  denom: string;
  [k: string]: unknown;
}
