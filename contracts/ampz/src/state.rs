use cosmwasm_std::{Addr, Order, StdResult, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use eris::{
    adapters::{
        arb_vault::ArbVault, compounder::Compounder, farm::Farm, generator::Generator, hub::Hub,
    },
    ampz::{AstroportConfig, CapapultConfig, Execution, FeeConfig, WhiteWhaleConfig},
};

use crate::error::ContractError;

pub(crate) struct State<'a> {
    // controller that can execute executions without receiving operation fees
    pub controller: Item<'a, Addr>,
    // addr of the zapper contract to execute multi swaps
    pub zapper: Item<'a, Compounder>,
    // config regarding supported astroport tokens and generator address
    pub astroport: Item<'a, AstroportConfig<Generator>>,
    // config regarding supported whitewhale
    pub whitewhale: Item<'a, WhiteWhaleConfig<Addr>>,

    pub capapult: Item<'a, CapapultConfig<Addr>>,

    /// Account who can call certain privileged functions
    pub owner: Item<'a, Addr>,
    /// Pending ownership transfer, awaiting acceptance by the new owner
    pub new_owner: Item<'a, Addr>,

    // address of the amplifier hub
    pub hub: Item<'a, Hub>,
    // address of the arb vault
    pub arb_vault: Item<'a, ArbVault>,
    // allowed farms to deposit
    pub farms: Item<'a, Vec<Farm>>,

    // next id for storing an execution
    pub id: Item<'a, u128>,
    // fee configuration
    pub fee: Item<'a, FeeConfig<Addr>>,

    // Runtime data
    // contains all executions info
    pub executions: IndexedMap<'a, u128, Execution, ExecutionIndexes<'a>>,
    // contains the timestamp when an execution was last executed
    pub last_execution: Map<'a, u128, u64>,

    pub execution_user_source: Map<'a, (String, String), u128>,

    // temporary state if something is executing
    pub is_executing: Item<'a, bool>,
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
            arb_vault: Item::new("arb_vault"),
            astroport: Item::new("astro_generator"),
            whitewhale: Item::new("whitewhale"),
            capapult: Item::new("capapult"),

            id: Item::new("id"),

            executions: IndexedMap::new("executions", execution_indexes),
            execution_user_source: Map::new("execution_user_source"),

            last_execution: Map::new("last_execution"),

            is_executing: Item::new("is_executing"),

            fee: Item::new("fee_config"),
        }
    }
}

impl<'a> State<'a> {
    pub fn assert_owner(&self, storage: &dyn Storage, sender: &Addr) -> Result<(), ContractError> {
        let owner = self.owner.load(storage)?;
        if *sender == owner {
            Ok(())
        } else {
            Err(ContractError::Unauthorized {})
        }
    }

    pub fn get_by_id(&self, storage: &dyn Storage, id: u128) -> Result<Execution, ContractError> {
        self.executions.load(storage, id).map_err(|_| ContractError::ExecutionNotFound(id))
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
