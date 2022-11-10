use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, QuerierWrapper, StdError, StdResult, Uint128, WasmMsg,
};

/// The maximum amount of voters that can be kicked at once from
pub const VOTERS_MAX_LIMIT: u32 = 30;

/// This structure describes the basic settings for creating a contract.
#[cw_serde]
pub struct InstantiateMsg {
    /// Contract owner
    pub owner: String,
    /// Factory contract address
    pub hub_addr: String,

    pub validators_limit: u64,
}

#[cw_serde]
pub struct EmpInfo {
    pub umerit_points: Uint128,
    pub decaying_period: Option<u64>,
}

// validator->points received
pub type AddEmpInfo = (String, Vec<EmpInfo>);

/// This structure describes the execute messages available in the contract.
#[cw_serde]
pub enum ExecuteMsg {
    /// Vote allows a vxASTRO holder to cast votes on which generators should get ASTRO emissions in the next epoch
    AddEmps {
        emps: Vec<AddEmpInfo>,
    },
    TuneEmps {},
    // RemoveEmps {
    //     emps: Vec<AddEmpInfo>,
    // },
    UpdateConfig {
        validators_limit: Option<u64>,
    },
    /// ProposeNewOwner proposes a new owner for the contract
    ProposeNewOwner {
        /// Newly proposed contract owner
        new_owner: String,
        /// The timestamp when the contract ownership change expires
        expires_in: u64,
    },
    /// DropOwnershipProposal removes the latest contract ownership transfer proposal
    DropOwnershipProposal {},
    /// ClaimOwnership allows the newly proposed owner to claim contract ownership
    ClaimOwnership {},
}

/// This structure describes the query messages available in the contract.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// TuneInfo returns information about the latest generators that were voted to receive ASTRO emissions
    #[returns(GaugeInfoResponse)]
    TuneInfo {},
    /// Config returns the contract configuration
    #[returns(ConfigResponse)]
    Config {},
    /// PoolInfo returns the latest voting power allocated to a specific pool (generator)
    #[returns(VotedValidatorInfoResponse)]
    ValidatorInfo {
        validator_addr: String,
    },
    /// PoolInfo returns the voting power allocated to a specific pool (generator) at a specific period
    #[returns(VotedValidatorInfoResponse)]
    ValidatorInfoAtPeriod {
        validator_addr: String,
        period: u64,
    },
    /// ValidatorInfos returns the latest EMPs allocated to all active validators
    #[returns(Vec<(String,VotedValidatorInfoResponse)>)]
    ValidatorInfos {
        validator_addrs: Option<Vec<String>>,
        period: Option<u64>,
    },
}

/// This structure describes a migration message.
/// We currently take no arguments for migrations.
#[cw_serde]
pub struct MigrateMsg {}

/// This structure describes the parameters returned when querying for the contract configuration.
#[cw_serde]
pub struct ConfigResponse {
    /// Address that's allowed to change contract parameters
    pub owner: Addr,
    /// Factory contract address
    pub hub_addr: Addr,

    pub validators_limit: u64,
}

impl ConfigResponse {
    pub fn assert_owner(&self, addr: &Addr) -> StdResult<()> {
        if *addr != self.owner {
            return Err(StdError::generic_err("unauthorized"));
        }
        Ok(())
    }

    pub fn assert_owner_or_self(&self, addr: &Addr, contract_addr: &Addr) -> StdResult<()> {
        if *addr != self.owner && *addr != *contract_addr {
            return Err(StdError::generic_err("unauthorized"));
        }
        Ok(())
    }
}

/// This structure describes the response used to return voting information for a specific pool (generator).
#[cw_serde]
#[derive(Default)]
pub struct VotedValidatorInfoResponse {
    /// Dynamic decaying power
    pub voting_power: Uint128,
    /// Fixed power
    pub fixed_amount: Uint128,
    /// The slope at which the amount of vxASTRO that voted for this pool/generator will decay
    pub slope: Uint128,
}

/// This structure describes the response used to return tuning parameters for all pools/generators.
#[cw_serde]
#[derive(Default)]
pub struct GaugeInfoResponse {
    /// Last timestamp when a tuning vote happened
    pub tune_ts: u64,
    /// Last period when a tuning vote happened
    #[serde(default)]
    pub tune_period: u64,
    /// Distribution of alloc_points to apply in the Generator contract
    pub emp_points: Vec<(String, Uint128)>,
}

/// Queries user's lockup information from the voting escrow contract.
///
/// * **user** staker for which we return lock position information.
pub fn get_tune_msg(contract_addr: impl Into<String>) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract_addr.into(),
        msg: to_binary(&ExecuteMsg::TuneEmps {})?,
        funds: vec![],
    }))
}

/// Queries emp tune info.
pub fn get_emp_tune_info(
    querier: &QuerierWrapper,
    escrow_addr: impl Into<String>,
) -> StdResult<GaugeInfoResponse> {
    let gauge: GaugeInfoResponse = querier.query_wasm_smart(escrow_addr, &QueryMsg::TuneInfo {})?;
    Ok(gauge)
}

pub fn get_emp_validator_infos(
    querier: &QuerierWrapper,
    emp_gauge_addr: impl Into<String>,
    period: u64,
) -> StdResult<Vec<(String, VotedValidatorInfoResponse)>> {
    let gauge: Vec<(String, VotedValidatorInfoResponse)> = querier.query_wasm_smart(
        emp_gauge_addr,
        &QueryMsg::ValidatorInfos {
            validator_addrs: None,
            period: Some(period),
        },
    )?;
    Ok(gauge)
}
