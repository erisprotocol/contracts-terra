use astroport::asset::Asset;
use cosmwasm_std::{to_binary, Addr, Coin, CosmosMsg, Env, QuerierWrapper, StdResult};
use cw20::{Cw20ExecuteMsg, Expiration};
use protobuf::SpecialFields;

use crate::{
    constants::CONTRACT_DENOM,
    protos::{msgex::MsgExecuteContractEx, proto::MsgExecuteContract},
    types::Delegation,
};

pub(crate) fn query_all_delegations(
    querier: &QuerierWrapper,
    delegator_addr: &Addr,
) -> StdResult<Vec<Delegation>> {
    let result: Vec<_> = querier
        .query_all_delegations(delegator_addr)?
        .into_iter()
        .filter(|d| d.amount.denom == CONTRACT_DENOM && !d.amount.amount.is_zero())
        .map(|d| Delegation {
            validator: d.validator,
            amount: d.amount.amount.u128(),
        })
        .collect();

    Ok(result)
}

pub fn funds_or_allowance(
    env: &Env,
    sender: &Addr,
    spender: &Addr,
    assets: &[Asset],
) -> StdResult<(Vec<Coin>, Vec<CosmosMsg>)> {
    let mut funds: Vec<Coin> = vec![];
    let mut msgs: Vec<CosmosMsg> = vec![];

    for asset in assets.iter() {
        if asset.is_native_token() {
            funds.push(cosmwasm_std::Coin {
                denom: asset.info.to_string(),
                amount: asset.amount,
            });
        } else {
            let execute_contract = MsgExecuteContract {
                sender: sender.to_string(),
                contract: asset.info.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: spender.to_string(),
                    amount: asset.amount,
                    expires: Some(Expiration::AtHeight(env.block.height + 1)),
                })?
                .to_vec(),
                funds: vec![],
                special_fields: SpecialFields::default(),
            };

            msgs.push(execute_contract.to_cosmos_msg(env)?);
        }
    }

    Ok((funds, msgs))
}
