use astroport::asset::{native_asset_info, token_asset_info, AssetInfo};
use cosmwasm_std::DepsMut;
use eris::ampz::{DepositMarket, Execution, RepayMarket};
use eris::constants::CONTRACT_DENOM;
use eris::{
    adapters::farm::Farm,
    ampz::{DestinationState, Source},
};

use crate::{
    error::{ContractError, CustomResult},
    helpers::validate_receiver,
    state::State,
};

pub(crate) trait ExecutionExt {
    fn validate(&self, deps: &DepsMut, state: &State) -> CustomResult<()>;
    fn get_source_assets(
        &self,
        deps: &DepsMut,
        state: &State,
        asset_info: Option<&AssetInfo>,
    ) -> CustomResult<Vec<AssetInfo>>;

    fn check_path_to(&self, deps: &DepsMut, state: &State, asset_info: AssetInfo) -> CustomResult;
}

impl ExecutionExt for Execution {
    fn validate(&self, deps: &DepsMut, state: &State) -> CustomResult<()> {
        match &self.destination {
            DestinationState::DepositAmplifier {
                receiver,
            } => {
                validate_receiver(deps.api, receiver)?;
            },
            DestinationState::DepositArbVault {
                receiver,
            } => {
                validate_receiver(deps.api, receiver)?;
            },
            DestinationState::DepositFarm {
                farm,
                receiver,
            } => {
                validate_receiver(deps.api, receiver)?;
                let allowed_farms = state.farms.load(deps.storage)?;
                let farm = Farm(deps.api.addr_validate(farm)?);
                if !allowed_farms.contains(&farm) {
                    return Err(ContractError::FarmNotSupported(farm.0.to_string()));
                }
            },
            DestinationState::SwapTo {
                asset_info,
                receiver,
            } => {
                validate_receiver(deps.api, receiver)?;
                // this checks if there is a configured route from the source asset to the destination asset
                let from_assets = self.get_source_assets(deps, state, Some(asset_info))?;

                let zapper = state.zapper.load(deps.storage)?;

                for from in from_assets {
                    if !zapper.query_support_swap(
                        &deps.querier,
                        from.clone(),
                        asset_info.clone(),
                    )? {
                        return Err(ContractError::SwapNotSupported(from, asset_info.clone()));
                    }
                }
            },
            DestinationState::Repay {
                market,
            } => match market {
                RepayMarket::Capapult => {
                    let capa = state.capapult.load(deps.storage)?;
                    self.check_path_to(deps, state, token_asset_info(capa.stable_cw))?;
                },
            },
            DestinationState::DepositCollateral {
                market,
            } => match market {
                DepositMarket::Capapult {
                    asset_info,
                } => {
                    if asset_info.is_native_token() {
                        return Err(ContractError::NotSupported {});
                    }
                    self.check_path_to(deps, state, asset_info.clone())?;
                },
            },
        }

        Ok(())
    }

    fn check_path_to(&self, deps: &DepsMut, state: &State, asset_info: AssetInfo) -> CustomResult {
        let from_assets = self.get_source_assets(deps, state, None)?;
        let zapper = state.zapper.load(deps.storage)?;

        for from in from_assets {
            if !zapper.query_support_swap(&deps.querier, from.clone(), asset_info.clone())? {
                return Err(ContractError::SwapNotSupported(from, asset_info));
            }
        }

        Ok(())
    }

    fn get_source_assets(
        &self,
        deps: &DepsMut,
        state: &State,
        asset_info: Option<&AssetInfo>,
    ) -> CustomResult<Vec<AssetInfo>> {
        let from_assets = match &self.source {
            Source::Claim => {
                let default_asset = native_asset_info(CONTRACT_DENOM.to_string());
                if asset_info.is_some() && *asset_info.unwrap() == default_asset {
                    // cant use claim (utoken) to swap to utoken (useless)
                    Err(ContractError::CannotSwapToSameToken {})?
                }

                // for claiming staking rewards only check the default chain denom
                vec![default_asset]
            },
            Source::AstroRewards {
                ..
            } => {
                // for astroport check that all possible reward coins are supported
                state.astroport.load(deps.storage)?.coins
            },
            Source::Wallet {
                over,
                ..
            } => {
                if asset_info.is_some() && *asset_info.unwrap() == over.info {
                    // cant use same input token to swap to token (useless)
                    Err(ContractError::CannotSwapToSameToken {})?
                }
                vec![over.info.clone()]
            },
        };
        Ok(from_assets)
    }
}
