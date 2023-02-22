use astroport::asset::AssetInfo;
use cosmwasm_std::{CosmosMsg, Decimal, Deps, Uint128};

use crate::error::CustomResult;

pub trait LsdAdapter {
    fn get_name(&self) -> &str;

    fn asset(&self) -> AssetInfo;

    fn unbond(&self, deps: &Deps, amount: Uint128) -> CustomResult<Vec<CosmosMsg>>;

    fn query_unbonding(&mut self, deps: &Deps) -> CustomResult<Uint128>;

    fn withdraw(&mut self, deps: &Deps, amount: Uint128) -> CustomResult<Vec<CosmosMsg>>;

    fn query_withdrawable(&mut self, deps: &Deps) -> CustomResult<Uint128>;

    fn query_factor_x_to_normal(&mut self, deps: &Deps) -> CustomResult<Decimal>;
}
