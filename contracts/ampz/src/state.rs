use cosmwasm_std::{Addr, Order, StdError, StdResult, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use eris::{
    adapters::{compounder::Compounder, farm::Farm, generator::Generator, hub::Hub},
    ampz::{AstroportConfig, Execution, FeeConfig},
};

pub(crate) struct State<'a> {
    pub controller: Item<'a, Addr>,
    pub zapper: Item<'a, Compounder>,
    pub astroport: Item<'a, AstroportConfig<Generator>>,
    /// Account who can call certain privileged functions
    pub owner: Item<'a, Addr>,
    /// Pending ownership transfer, awaiting acceptance by the new owner
    pub new_owner: Item<'a, Addr>,

    pub hub: Item<'a, Hub>,
    pub farms: Item<'a, Vec<Farm>>,

    pub id: Item<'a, u128>,
    pub executions: IndexedMap<'a, u128, Execution, ExecutionIndexes<'a>>,
    pub last_execution: Map<'a, u128, u64>,
    pub execution_user_source: Map<'a, (String, String), u128>,

    pub is_executing: Item<'a, bool>,

    pub fee: Item<'a, FeeConfig<Addr>>, // pub tips: Item<'a, TipConfig>,
                                        // pub tip_jar: Map<'a, String, Uint128>,
}

impl Default for State<'static> {
    fn default() -> Self {
        let execution_indexes = ExecutionIndexes {
            user: MultiIndex::new(|d: &Execution| d.user.clone(), "executions", "executions__user"),
        };

        Self {
            owner: Item::new("owner"),
            zapper: Item::new("zapper"),
            controller: Item::new("controller"),
            new_owner: Item::new("new_owner"),
            farms: Item::new("farms"),
            hub: Item::new("hub"),
            astroport: Item::new("astro_generator"),

            id: Item::new("id"),
            executions: IndexedMap::new("executions", execution_indexes),
            execution_user_source: Map::new("execution_user_source"),
            last_execution: Map::new("last_execution"),

            // temporary state
            is_executing: Item::new("is_executing"),

            fee: Item::new("fee_config"),
            // tips: Item::new("tips"),
            // tip_jar: Map::new("tip_jar"),
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

    pub fn get_by_id(&self, storage: &dyn Storage, id: u128) -> StdResult<Execution> {
        self.executions
            .load(storage, id)
            .map_err(|_| StdError::generic_err(format!("could not find execution with id {}", id)))
    }

    pub fn get_by_user(
        &self,
        storage: &dyn Storage,
        user: String,
    ) -> StdResult<Vec<(u128, Execution)>> {
        self.executions
            .idx
            .user
            .prefix(user)
            .range(storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
    }
}

pub(crate) struct ExecutionIndexes<'a> {
    // pk goes to second tuple element
    pub user: MultiIndex<'a, String, Execution, u128>,
}

impl<'a> IndexList<Execution> for ExecutionIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Execution>> + '_> {
        let v: Vec<&dyn Index<Execution>> = vec![&self.user];

        Box::new(v.into_iter())
    }
}
