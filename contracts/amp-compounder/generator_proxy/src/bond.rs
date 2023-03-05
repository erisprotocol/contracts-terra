use crate::astro_generator::GeneratorEx;
use crate::error::ContractError;
use crate::model::{CallbackMsg, Config, PoolInfo, RewardInfo, UserInfo};
use crate::state::{CONFIG, POOL_INFO, REWARD_INFO, USER_INFO};
use astroport::asset::{token_asset, Asset};
use astroport::generator::{PendingTokenResponse, UserInfoV2};
use astroport::querier::query_token_balance;
use astroport::restricted_vector::RestrictedVector;
use cosmwasm_std::{
    attr, Addr, Attribute, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, QuerierWrapper,
    Response, StdError, StdResult, Uint128,
};
use eris::adapters::asset::AssetEx;
use std::cmp;
use std::collections::HashMap;

pub fn execute_deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    staker_addr: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // reward cannot be claimed if there is no record
    let mut messages: Vec<CosmosMsg> = vec![];
    let mut attributes: Vec<Attribute> = vec![];
    let config = CONFIG.load(deps.storage)?;
    let astro_user_info =
        config.generator.query_user_info(&deps.querier, &info.sender, &env.contract.address)?;
    if let Some(astro_user_info) = astro_user_info {
        let (claim, prev_balances) =
            reconcile_claimed_by_others(deps, &env, &config, &info.sender, &astro_user_info)?;
        if claim {
            let lp_token = info.sender.to_string();
            attributes = prev_balances
                .iter()
                .map(|bal| attr("prev_balance", format!("{0}{1}", bal.1, bal.0)))
                .collect::<Vec<Attribute>>();
            messages.push(config.generator.claim_rewards_msg(vec![lp_token])?);
            messages.push(
                CallbackMsg::AfterBondClaimed {
                    lp_token: info.sender.clone(),
                    prev_balances,
                }
                .to_cosmos_msg(&env.contract.address)?,
            );
        }
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_message(
            CallbackMsg::Deposit {
                lp_token: info.sender,
                staker_addr,
                amount,
            }
            .to_cosmos_msg(&env.contract.address)?,
        )
        .add_attribute("action", "ampg/deposit")
        .add_attributes(attributes))
}

pub fn execute_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    lp_token: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let lp_token = deps.api.addr_validate(&lp_token)?;
    let config = CONFIG.load(deps.storage)?;
    let astro_user_info = config
        .generator
        .query_user_info(&deps.querier, &lp_token, &env.contract.address)?
        .ok_or_else(|| StdError::generic_err("UserInfo is not found"))?;
    let (claim, prev_balances) =
        reconcile_claimed_by_others(deps, &env, &config, &lp_token, &astro_user_info)?;

    let mut messages: Vec<CosmosMsg> = vec![];
    if claim {
        messages.push(config.generator.claim_rewards_msg(vec![lp_token.to_string()])?);
        messages.push(
            CallbackMsg::AfterBondClaimed {
                lp_token: lp_token.clone(),
                prev_balances,
            }
            .to_cosmos_msg(&env.contract.address)?,
        );
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_message(
            CallbackMsg::Withdraw {
                lp_token,
                staker_addr: info.sender,
                amount,
            }
            .to_cosmos_msg(&env.contract.address)?,
        )
        .add_attribute("action", "ampg/withdraw"))
}

pub fn execute_claim_rewards(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    lp_tokens: Vec<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let mut messages: Vec<CosmosMsg> = vec![];

    for lp_token in lp_tokens {
        let lp_token = deps.api.addr_validate(&lp_token)?;
        let astro_user_info = config
            .generator
            .query_user_info(&deps.querier, &lp_token, &env.contract.address)?
            .ok_or_else(|| StdError::generic_err("UserInfo is not found"))?;
        let (claim, prev_balances) =
            reconcile_claimed_by_others(deps.branch(), &env, &config, &lp_token, &astro_user_info)?;
        if claim {
            messages.push(config.generator.claim_rewards_msg(vec![lp_token.to_string()])?);
            messages.push(
                CallbackMsg::AfterBondClaimed {
                    lp_token: lp_token.clone(),
                    prev_balances,
                }
                .to_cosmos_msg(&env.contract.address)?,
            );
        }
        messages.push(
            CallbackMsg::ClaimRewards {
                lp_token,
                staker_addr: info.sender.clone(),
            }
            .to_cosmos_msg(&env.contract.address)?,
        );
    }

    Ok(Response::new().add_messages(messages).add_attribute("action", "ampg/claim_rewards"))
}

fn fetch_balance(
    querier: &QuerierWrapper,
    config: &Config,
    contract_addr: &Addr,
    astro_user_info: &UserInfoV2,
) -> StdResult<Vec<(Addr, Uint128)>> {
    let astro_amount = query_token_balance(querier, &config.astro_token, contract_addr)?;
    let mut balances: Vec<(Addr, Uint128)> = vec![(config.astro_token.clone(), astro_amount)];
    for (token, _) in astro_user_info.reward_debt_proxy.inner_ref() {
        let token_amount = query_token_balance(querier, token, contract_addr)?;
        balances.push((token.clone(), token_amount));
    }
    Ok(balances)
}

fn reconcile_claimed_by_others(
    deps: DepsMut,
    env: &Env,
    config: &Config,
    lp_token: &Addr,
    astro_user_info: &UserInfoV2,
) -> StdResult<(bool, Vec<(Addr, Uint128)>)> {
    // load
    let pool_info_op = POOL_INFO
        .may_load(deps.storage, lp_token)?
        .filter(|pool_info| !pool_info.total_bond_share.is_zero());
    let mut pool_info = match pool_info_op {
        None => {
            let balances =
                fetch_balance(&deps.querier, config, &env.contract.address, astro_user_info)?;
            return Ok((true, balances));
        },
        Some(pool_info) if pool_info.last_reconcile == env.block.height => {
            let balances =
                fetch_balance(&deps.querier, config, &env.contract.address, astro_user_info)?;
            return Ok((false, balances));
        },
        Some(pool_info) => pool_info,
    };

    // reconcile astro
    let mut astro_reward =
        REWARD_INFO.may_load(deps.storage, &config.astro_token)?.unwrap_or_default();
    let astro_amount =
        query_token_balance(&deps.querier, &config.astro_token, &env.contract.address)?;
    let add_astro_amount = astro_amount.saturating_sub(astro_reward.reconciled_amount);
    let target_add_astro_amount = (astro_user_info.reward_user_index
        - pool_info.prev_reward_user_index)
        * astro_user_info.virtual_amount;
    let net_astro_amount = cmp::min(add_astro_amount, target_add_astro_amount);
    if !net_astro_amount.is_zero() {
        reconcile_astro_reward(
            config,
            astro_user_info,
            &mut pool_info,
            &mut astro_reward,
            net_astro_amount,
        )?;
        REWARD_INFO.save(deps.storage, &config.astro_token, &astro_reward)?;
    }

    // track balances
    let mut balances = vec![(config.astro_token.clone(), astro_amount)];

    // reconcile other tokens
    let rewards_debt_map: HashMap<_, _> =
        pool_info.prev_reward_debt_proxy.inner_ref().iter().cloned().collect();
    for (token, debt) in astro_user_info.reward_debt_proxy.inner_ref() {
        let mut token_reward = REWARD_INFO.may_load(deps.storage, token)?.unwrap_or_default();
        let prev_debt = rewards_debt_map.get(token).cloned().unwrap_or_default();
        let target_add_token_amount = debt.saturating_sub(prev_debt);

        let token_amount = query_token_balance(&deps.querier, token, &env.contract.address)?;
        let add_token_amount = token_amount.saturating_sub(token_reward.reconciled_amount);
        let net_token_amount = cmp::min(add_token_amount, target_add_token_amount);
        if !net_token_amount.is_zero() {
            reconcile_token_reward(token, &mut pool_info, &mut token_reward, net_token_amount)?;
            REWARD_INFO.save(deps.storage, token, &token_reward)?;
        }

        balances.push((token.clone(), token_amount));
    }

    // set index and save
    pool_info.prev_reward_user_index = astro_user_info.reward_user_index;
    pool_info.prev_reward_debt_proxy = astro_user_info.reward_debt_proxy.clone();
    POOL_INFO.save(deps.storage, lp_token, &pool_info)?;

    Ok((true, balances))
}

fn reconcile_astro_reward(
    config: &Config,
    astro_user_info: &UserInfoV2,
    pool_info: &mut PoolInfo,
    astro_reward: &mut RewardInfo,
    net_astro_amount: Uint128,
) -> StdResult<()> {
    let based_astro = net_astro_amount.multiply_ratio(
        astro_user_info.amount * Decimal::percent(40),
        astro_user_info.virtual_amount,
    );
    let boosted_astro = net_astro_amount.checked_sub(based_astro)?;
    let fee = boosted_astro * config.boost_fee;
    let net_boosted_astro = boosted_astro - fee;
    let to_staker = net_boosted_astro * config.staker_rate;
    let to_lp = net_boosted_astro - to_staker + based_astro;
    let astro_per_share = Decimal::from_ratio(to_lp, pool_info.total_bond_share);
    astro_reward.fee += fee;
    astro_reward.staker_income += to_staker;
    astro_reward.reconciled_amount += net_astro_amount;
    pool_info.reward_indexes.update(&config.astro_token, astro_per_share)?;

    Ok(())
}

fn reconcile_token_reward(
    token: &Addr,
    pool_info: &mut PoolInfo,
    token_reward: &mut RewardInfo,
    net_token_amount: Uint128,
) -> StdResult<()> {
    let token_per_share = Decimal::from_ratio(net_token_amount, pool_info.total_bond_share);
    token_reward.reconciled_amount += net_token_amount;
    pool_info.reward_indexes.update(token, token_per_share)?;

    Ok(())
}

pub fn callback_after_bond_claimed(
    deps: DepsMut,
    env: Env,
    lp_token: Addr,
    prev_balances: Vec<(Addr, Uint128)>,
) -> Result<Response, ContractError> {
    // load
    let config = CONFIG.load(deps.storage)?;
    let mut pool_info = POOL_INFO.load(deps.storage, &lp_token)?;
    let astro_user_info = config
        .generator
        .query_user_info(&deps.querier, &lp_token, &env.contract.address)?
        .ok_or_else(|| StdError::generic_err("UserInfo not found"))?;

    // reconcile astro
    let mut astro_reward =
        REWARD_INFO.may_load(deps.storage, &config.astro_token)?.unwrap_or_default();
    let prev_balance_map: HashMap<_, _> = prev_balances.into_iter().collect();
    let astro_amount =
        query_token_balance(&deps.querier, &config.astro_token, &env.contract.address)?;
    if let Some(prev_astro_amount) = prev_balance_map.get(&config.astro_token) {
        let net_astro_amount = astro_amount.checked_sub(*prev_astro_amount)?;
        if !net_astro_amount.is_zero() {
            reconcile_astro_reward(
                &config,
                &astro_user_info,
                &mut pool_info,
                &mut astro_reward,
                net_astro_amount,
            )?;
            REWARD_INFO.save(deps.storage, &config.astro_token, &astro_reward)?;
        }
    }

    let mut attributes = vec![];

    // reconcile other tokens
    for (token, debited) in astro_user_info.reward_debt_proxy.inner_ref() {
        attributes.push(attr("token", token));
        attributes.push(attr("debited", debited.to_string()));
        if let Some(prev_token_amount) = prev_balance_map.get(token) {
            let mut token_reward = REWARD_INFO.may_load(deps.storage, token)?.unwrap_or_default();
            let token_amount = query_token_balance(&deps.querier, token, &env.contract.address)?;
            let net_token_amount = token_amount.checked_sub(*prev_token_amount)?;
            attributes.push(attr("prev_balance", prev_token_amount.to_string()));
            attributes.push(attr("token_amount", token_amount));
            attributes.push(attr("net_token_amount", net_token_amount));
            if !net_token_amount.is_zero() {
                reconcile_token_reward(token, &mut pool_info, &mut token_reward, net_token_amount)?;
                REWARD_INFO.save(deps.storage, token, &token_reward)?;
            }
        }
    }

    // set index and save
    pool_info.prev_reward_user_index = astro_user_info.reward_user_index;
    pool_info.prev_reward_debt_proxy = astro_user_info.reward_debt_proxy;
    pool_info.last_reconcile = env.block.height;
    POOL_INFO.save(deps.storage, &lp_token, &pool_info)?;

    Ok(Response::new()
        .add_attribute("action", "ampg/after_bond_claimed")
        .add_attributes(attributes))
}

pub fn callback_after_bond_changed(
    deps: DepsMut,
    env: Env,
    lp_token: Addr,
) -> Result<Response, ContractError> {
    // load
    let config = CONFIG.load(deps.storage)?;

    // debt will reset after share changed
    if let Some(astro_user_info) =
        config.generator.query_user_info(&deps.querier, &lp_token, &env.contract.address)?
    {
        // set index and save
        let mut pool_info = POOL_INFO.load(deps.storage, &lp_token)?;
        pool_info.prev_reward_user_index = astro_user_info.reward_user_index;
        pool_info.prev_reward_debt_proxy = astro_user_info.reward_debt_proxy;
        POOL_INFO.save(deps.storage, &lp_token, &pool_info)?;
    }

    Ok(Response::default().add_attribute("callback", "ampg/after_bond_changed"))
}

pub fn reconcile_to_user_info(pool_info: &PoolInfo, user_info: &mut UserInfo) -> StdResult<()> {
    let user_indexes: HashMap<_, _> =
        user_info.reward_indexes.inner_ref().iter().cloned().collect();
    for (token, index) in pool_info.reward_indexes.inner_ref() {
        let user_index = user_indexes.get(token).cloned().unwrap_or_default();
        let amount = (*index - user_index) * user_info.bond_share;
        user_info.pending_rewards.update(token, amount)?;
    }
    user_info.reward_indexes = pool_info.reward_indexes.clone();

    Ok(())
}

pub fn callback_deposit(
    deps: DepsMut,
    env: Env,
    lp_token: Addr,
    staker_addr: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // load
    let config = CONFIG.load(deps.storage)?;
    let mut pool_info = POOL_INFO.may_load(deps.storage, &lp_token)?.unwrap_or_default();
    let mut user_info = USER_INFO
        .may_load(deps.storage, (&lp_token, &staker_addr))?
        .unwrap_or_else(|| UserInfo::create(&pool_info));

    // update
    reconcile_to_user_info(&pool_info, &mut user_info)?;
    let total_bond_amount =
        config.generator.query_deposit(&deps.querier, &lp_token, &env.contract.address)?;
    let share = pool_info.calc_bond_share(total_bond_amount, amount, false);
    user_info.bond_share += share;
    pool_info.total_bond_share += share;

    // save
    USER_INFO.save(deps.storage, (&lp_token, &staker_addr), &user_info)?;
    POOL_INFO.save(deps.storage, &lp_token, &pool_info)?;

    let deposit_msg = config.generator.deposit_msg(lp_token.to_string(), amount)?;
    Ok(Response::new()
        .add_message(deposit_msg)
        .add_message(
            CallbackMsg::AfterBondChanged {
                lp_token,
            }
            .to_cosmos_msg(&env.contract.address)?,
        )
        .add_attribute("add_share", share))
}

pub fn callback_withdraw(
    deps: DepsMut,
    env: Env,
    lp_token: Addr,
    staker_addr: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // load
    let config = CONFIG.load(deps.storage)?;
    let mut pool_info = POOL_INFO.load(deps.storage, &lp_token)?;
    let mut user_info = USER_INFO.load(deps.storage, (&lp_token, &staker_addr))?;

    // update
    reconcile_to_user_info(&pool_info, &mut user_info)?;
    let total_bond_amount =
        config.generator.query_deposit(&deps.querier, &lp_token, &env.contract.address)?;
    let share = pool_info.calc_bond_share(total_bond_amount, amount, true);
    user_info.bond_share = user_info.bond_share.checked_sub(share)?;
    pool_info.total_bond_share = pool_info.total_bond_share.checked_sub(share)?;

    // save
    USER_INFO.save(deps.storage, (&lp_token, &staker_addr), &user_info)?;
    POOL_INFO.save(deps.storage, &lp_token, &pool_info)?;

    let withdraw_msg = config.generator.withdraw_msg(lp_token.to_string(), amount)?;
    Ok(Response::new()
        .add_message(withdraw_msg)
        .add_message(token_asset(lp_token.clone(), amount).transfer_msg(&staker_addr)?)
        .add_message(
            CallbackMsg::AfterBondChanged {
                lp_token,
            }
            .to_cosmos_msg(&env.contract.address)?,
        )
        .add_attribute("deduct_share", share))
}

pub fn callback_claim_rewards(
    deps: DepsMut,
    _env: Env,
    lp_token: Addr,
    staker_addr: Addr,
) -> Result<Response, ContractError> {
    // load
    let mut user_info = USER_INFO.load(deps.storage, (&lp_token, &staker_addr))?;
    let pool_info = POOL_INFO.load(deps.storage, &lp_token)?;
    reconcile_to_user_info(&pool_info, &mut user_info)?;

    // send
    let mut messages: Vec<CosmosMsg> = vec![];
    for (token, amount) in user_info.pending_rewards.inner_ref() {
        if amount.is_zero() {
            continue;
        }

        let mut reward_info = REWARD_INFO.load(deps.storage, token)?;
        reward_info.reconciled_amount = reward_info.reconciled_amount.checked_sub(*amount)?;
        REWARD_INFO.save(deps.storage, token, &reward_info)?;

        let asset = token_asset(token.clone(), *amount);
        messages.push(asset.transfer_msg(&staker_addr)?);
    }
    user_info.pending_rewards = RestrictedVector::default();

    // save
    USER_INFO.save(deps.storage, (&lp_token, &staker_addr), &user_info)?;

    Ok(Response::new().add_messages(messages))
}

pub fn query_pending_token(
    deps: Deps,
    env: Env,
    lp_token: String,
    user: String,
) -> Result<PendingTokenResponse, ContractError> {
    // load
    let lp_token = deps.api.addr_validate(&lp_token)?;
    let user = deps.api.addr_validate(&user)?;
    let config = CONFIG.load(deps.storage)?;
    let astro_user_info =
        match config.generator.query_user_info(&deps.querier, &lp_token, &env.contract.address)? {
            Some(astro_user_info) => astro_user_info,
            None => {
                return Ok(PendingTokenResponse {
                    pending: Uint128::zero(),
                    pending_on_proxy: None,
                });
            },
        };
    let mut pool_info = POOL_INFO.may_load(deps.storage, &lp_token)?.unwrap_or_default();
    let mut user_info = USER_INFO
        .may_load(deps.storage, (&lp_token, &user))?
        .unwrap_or_else(|| UserInfo::create(&pool_info));
    let pending_token =
        config.generator.query_pending_token(&deps.querier, &lp_token, &env.contract.address)?;

    // reconcile astro
    let mut astro_reward =
        REWARD_INFO.may_load(deps.storage, &config.astro_token)?.unwrap_or_default();
    let astro_amount =
        query_token_balance(&deps.querier, &config.astro_token, &env.contract.address)?;
    let add_astro_amount = astro_amount.saturating_sub(astro_reward.reconciled_amount);
    let target_add_astro_amount = (astro_user_info.reward_user_index
        - pool_info.prev_reward_user_index)
        * astro_user_info.virtual_amount;
    let net_astro_amount =
        cmp::min(add_astro_amount, target_add_astro_amount) + pending_token.pending;
    reconcile_astro_reward(
        &config,
        &astro_user_info,
        &mut pool_info,
        &mut astro_reward,
        net_astro_amount,
    )?;

    // reconcile other tokens
    let rewards_debt_map: HashMap<_, _> =
        pool_info.prev_reward_debt_proxy.inner_ref().iter().cloned().collect();
    let pending_token_map: HashMap<_, _> = if let Some(tokens) = pending_token.pending_on_proxy {
        tokens.into_iter().map(|it| (it.info.to_string(), it.amount)).collect()
    } else {
        HashMap::new()
    };
    for (token, debt) in astro_user_info.reward_debt_proxy.inner_ref() {
        let mut token_reward = REWARD_INFO.may_load(deps.storage, token)?.unwrap_or_default();
        let prev_debt = rewards_debt_map.get(token).cloned().unwrap_or_default();
        let target_add_token_amount = debt.saturating_sub(prev_debt);
        let add_pending_amount =
            pending_token_map.get(&token.to_string()).cloned().unwrap_or_default();

        let token_amount = query_token_balance(&deps.querier, token, &env.contract.address)?;
        let add_token_amount = token_amount.saturating_sub(token_reward.reconciled_amount);
        let net_token_amount =
            cmp::min(add_token_amount, target_add_token_amount) + add_pending_amount;
        reconcile_token_reward(token, &mut pool_info, &mut token_reward, net_token_amount)?;
    }
    pool_info.prev_reward_debt_proxy = astro_user_info.reward_debt_proxy;

    // reconcile to user info
    reconcile_to_user_info(&pool_info, &mut user_info)?;

    // build data
    let mut pending = Uint128::zero();
    let mut pending_on_proxy: Vec<Asset> = vec![];
    for (addr, amount) in user_info.pending_rewards.inner_ref() {
        if addr == &config.astro_token {
            pending = *amount;
        } else {
            pending_on_proxy.push(token_asset(addr.clone(), *amount));
        }
    }

    Ok(PendingTokenResponse {
        pending,
        pending_on_proxy: if pending_on_proxy.is_empty() {
            None
        } else {
            Some(pending_on_proxy)
        },
    })
}

pub fn query_deposit(
    deps: Deps,
    env: Env,
    lp_token: String,
    user: String,
) -> Result<Uint128, ContractError> {
    // load
    let lp_token = deps.api.addr_validate(&lp_token)?;
    let user = deps.api.addr_validate(&user)?;
    let config = CONFIG.load(deps.storage)?;
    let pool_info = POOL_INFO.may_load(deps.storage, &lp_token)?.unwrap_or_default();
    let user_info = USER_INFO
        .may_load(deps.storage, (&lp_token, &user))?
        .unwrap_or_else(|| UserInfo::create(&pool_info));

    // query
    let total_bond_amount =
        config.generator.query_deposit(&deps.querier, &lp_token, &env.contract.address)?;
    let user_bond_amount = pool_info.calc_bond_amount(total_bond_amount, user_info.bond_share);
    Ok(user_bond_amount)
}
