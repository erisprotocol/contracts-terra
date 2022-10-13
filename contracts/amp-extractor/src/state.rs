use cosmwasm_std::{Addr, Decimal, StdError, StdResult, Storage, Uint128};
use cw_storage_plus::Item;

use eris::amp_extractor::ExtractConfig;

pub struct State<'a> {
    /// Account who can call certain privileged functions
    pub owner: Item<'a, Addr>,
    /// Pending ownership transfer, awaiting acceptance by the new owner
    pub new_owner: Item<'a, Addr>,
    /// Address of the Liquid Staking token
    pub lp_token: Item<'a, Addr>,
    /// Address of the Staking token
    pub stake_token: Item<'a, Addr>,
    /// Extract Config
    pub extract_config: Item<'a, ExtractConfig>,

    pub stake_extracted: Item<'a, Uint128>,
    pub stake_harvested: Item<'a, Uint128>,
    pub last_exchange_rate: Item<'a, Decimal>,
}

impl Default for State<'static> {
    fn default() -> Self {
        Self {
            owner: Item::new("owner"),
            new_owner: Item::new("new_owner"),
            lp_token: Item::new("lp_token"),
            stake_token: Item::new("stake_token"),
            extract_config: Item::new("extract_config"),

            stake_extracted: Item::new("stake_extracted"),
            stake_harvested: Item::new("stake_harvested"),
            last_exchange_rate: Item::new("last_exchange_rate"),
        }
    }
}

impl<'a> State<'a> {
    pub fn assert_owner(&self, storage: &dyn Storage, sender: &Addr) -> StdResult<()> {
        let owner = self.owner.load(storage)?;
        if *sender == owner {
            Ok(())
        } else {
            Err(StdError::generic_err("unauthorized: sender is not owner"))
        }
    }
}
