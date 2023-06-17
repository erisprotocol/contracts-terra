# Eris Stake Hub (Amplified [Token])

Eris Stake Hub contract manages the bonding/unbonding of [Token], minting/burning of ampLUNA, and reinvestment of staking rewards.

## Overview

### Exchange rate

Unlike [Lido's stETH](https://github.com/lidofinance/lido-dao/tree/master/contracts/0.4.24), the ampLUNA token does not rebase; instead, the exchange rate between [Token] and ampLUNA increases (i.e. each ampLUNA becomes worth more [Token]) as staking rewards are harvested, and reduces if validators are slashed.

The exchange rate, as defined by the amount of `utoken` redeemable per `ustake`, is calculated as

```plain
exchange_rate = total_utoken_staked / total_ustake_supply
```

### Unlocked coins

Unlocked coin refers to coins held by the Eris Staking Hub contract (referred to as "the contract" hereafter) that can be reinvested. The contract tracks the amounts of unlocked coins using a `Vec<cosmwasm_std::Coin>` variable stored under the `unlocked_coins` key.

Each time the Hub contract delegates to or undelegates from a validator, the claimable staking rewards are automatically transferred to the contract. The amounts of coins transferred are recorded in the `coin_received` event. When handling the response, the contract parses this event and updates the `unlocked_coins` variable accordingly.

When harvesting, the contract needs to swap Terra stablecoins into [Token]. the contract offers all unlocked coins that have exchange rates defined against [Token] to be swapped, and deduct them from `unlocked_coins` accordingly. When handling the response, the contract parses the `swap` event and increments the unlocked [Token] amount.

### Unbonding

Cosmos chains, by default, has a limit of 7 undelegations at a time per validator-delegator pair. In order to support unbonding requests from many users, the contract needs to bundle unbonding requests together and submit them in batches.

![illustration-of-unbonding-queue](./unbonding-queue.png)

For mainnet, the contract will submit a batch every 3 days, such that there are at most 7 undelegations at a time with each validator. This 3 day interval is defined by the `epoch_period` parameter.

During the 3 day period, the contract accepts unbonding requests from users and store them in an `IndexedMap` data structure under the `unbond_requests` key, and the aggregated properties of the pending batch under the `pending_batch` key. Each user's share in the batch is proportional to the amount of ampLUNA tokens the user requests to burn.

At the end of the 3 day period, anyone can invoke the `ExecuteMsg::SubmitBatch` function to submit the pending batch to be unbonded. The contract calculates the amount of [Token] to unbond based on the [Token]/ampLUNA exchange rate at the time, burns the ampLUNA tokens, and initiates undelegations with the validators.

At the end of the following 21 day unbonding period, the user can invoke the `ExecuteMsg::WithdrawUnbonded` function. The contract pulls all of the user's unclaimed unbonding requests, and refunds appropriate amounts of [Token] based on the each request's share in that batch, to the user.

## Reference

Similar projects:

* [Lido - stLUNA](https://github.com/lidofinance/lido-terra-contracts)
* [Stader - [Token]X](https://github.com/stader-labs/stader-liquid-token)
* [Prism - cLUNA (not published anymore)](https://github.com/prism-finance)
* [Staking derivatives (dSCRT)](https://github.com/Cashmaney/SecretStaking)
