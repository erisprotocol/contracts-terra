use crate::model::{Config, PoolInfo, RewardInfo, StakerInfo, StakingState, UserInfo};
use astroport::common::OwnershipProposal;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

/// Stores the contract config
pub const CONFIG: Item<Config> = Item::new("config");

/// Stores pool info per LP token, key = LP token
pub const POOL_INFO: Map<&Addr, PoolInfo> = Map::new("pool_info");

/// Stores user info per user per LP token, key = LP token, User
pub const USER_INFO: Map<(&Addr, &Addr), UserInfo> = Map::new("user_info");

/// Stores reward info per reward token, key = Reward Token
pub const REWARD_INFO: Map<&Addr, RewardInfo> = Map::new("reward_info");

/// Stores the contract state for staking related
pub const STAKING_STATE: Item<StakingState> = Item::new("staking_state");

/// Stores staker info per user, key = User
pub const STAKER_INFO: Map<&Addr, StakerInfo> = Map::new("staker_info");

/// Stores the latest proposal to change contract ownership
pub const OWNERSHIP_PROPOSAL: Item<OwnershipProposal> = Item::new("ownership_proposal");
