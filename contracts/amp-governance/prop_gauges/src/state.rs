use astroport::common::OwnershipProposal;

use cosmwasm_std::{Addr, Env, StdResult, Storage, VoteOption};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use eris::prop_gauges::{ConfigResponse, PropInfo, PropUserInfo};

pub type Config = ConfigResponse;

pub(crate) struct State<'a> {
    pub config: Item<'a, Config>,
    pub props: IndexedMap<'a, u64, PropInfo, PropsIndexes<'a>>,
    pub ownership_proposal: Item<'a, OwnershipProposal>,
    pub users: IndexedMap<'a, (u64, Addr), PropUserInfo, UserIndexes<'a>>,
    pub voters: Map<'a, (u64, u128, Addr), VoteOption>,
}

impl Default for State<'static> {
    fn default() -> Self {
        let props_indexes = PropsIndexes {
            time: MultiIndex::new(|d: &PropInfo| d.end_time_s, "props", "props__time"),
        };
        let user_indexes = UserIndexes {
            user: MultiIndex::new(|d: &PropUserInfo| d.user.clone(), "users", "users__user"),
        };

        Self {
            config: Item::new("config"),
            props: IndexedMap::new("props", props_indexes),
            ownership_proposal: Item::new("ownership_proposal"),
            users: IndexedMap::new("users", user_indexes),
            voters: Map::new("voters"),
        }
    }
}

pub(crate) struct PropsIndexes<'a> {
    pub time: MultiIndex<'a, u64, PropInfo, u64>,
}

impl<'a> IndexList<PropInfo> for PropsIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<PropInfo>> + '_> {
        let v: Vec<&dyn Index<PropInfo>> = vec![&self.time];
        Box::new(v.into_iter())
    }
}

pub(crate) struct UserIndexes<'a> {
    pub user: MultiIndex<'a, Addr, PropUserInfo, (u64, Addr)>,
}

impl<'a> IndexList<PropUserInfo> for UserIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<PropUserInfo>> + '_> {
        let v: Vec<&dyn Index<PropUserInfo>> = vec![&self.user];
        Box::new(v.into_iter())
    }
}

impl<'a> State<'a> {
    pub fn get_user_info(
        &self,
        store: &dyn Storage,
        id: u64,
        addr: &Addr,
    ) -> StdResult<Option<PropUserInfo>> {
        // let func = self.user_info;
        // let user_info = func(id);
        // user_info.may_load(storage, &addr)

        self.users.may_load(store, (id, addr.clone()))
    }

    pub fn all_active_props(
        &self,
        store: &dyn Storage,
        env: &Env,
    ) -> StdResult<Vec<(u64, PropInfo)>> {
        let current_time = env.block.time.seconds();
        let start = Some(Bound::inclusive((current_time, 0)));

        let props = self
            .props
            .idx
            .time
            .range(store, start, None, cosmwasm_std::Order::Ascending)
            .collect::<StdResult<Vec<(u64, PropInfo)>>>()?;

        Ok(props)
    }
}
