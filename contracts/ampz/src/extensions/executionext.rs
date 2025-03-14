use astroport::asset::{native_asset_info, token_asset_info, AssetInfo};
use cosmwasm_std::DepsMut;
use eris::ampz::{DepositMarket, Execution, RepayMarket};
use eris::{
    adapters::farm::Farm,
    ampz::{DestinationState, Source},
};

use crate::constants::WW_MIN_LOCK_TIME;
use crate::{
    constants::CONTRACT_DENOM,
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
            DestinationState::ExecuteContract {
                contract,
                ..
            } => {
                deps.api.addr_validate(contract.as_str())?;
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
            DestinationState::DepositLiquidity {
                lp_token,
                dex,
            } => match dex {
                eris::ampz::DepositLiquidity::WhiteWhale {
                    lock_up,
                } => {
                    let whitewhale = state.whitewhale.load(deps.storage)?;
                    let lp_token = deps.api.addr_validate(lp_token)?;
                    if !whitewhale.lp_tokens.contains(&lp_token) {
                        return Err(ContractError::LpTokenNotSupported(lp_token.to_string()));
                    }

                    if let Some(lock_up) = lock_up {
                        if *lock_up < WW_MIN_LOCK_TIME {
                            return Err(ContractError::LockTimeTooShort {});
                        }
                    }
                },
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
            DestinationState::DepositTAmplifier {
                receiver,
                asset_info,
            } => {
                validate_receiver(deps.api, receiver)?;
                self.check_path_to(deps, state, asset_info.clone())?;
            },

            DestinationState::LiquidityAlliance {
                gauge,
                lp_info,
                compounding,
            } => todo!(),
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
                    // cant use claim (uluna) to swap to uluna (useless)
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
            Source::ClaimContract {
                claim_type,
            } => match claim_type {
                eris::ampz::ClaimType::WhiteWhaleRewards => {
                    state.whitewhale.load(deps.storage)?.coins
                },
                eris::ampz::ClaimType::AllianceRewards => {
                    let default_asset = native_asset_info(CONTRACT_DENOM.to_string());
                    if asset_info.is_some() && *asset_info.unwrap() == default_asset {
                        // cant use claim (uluna) to swap to uluna (useless)
                        Err(ContractError::CannotSwapToSameToken {})?
                    }

                    // for claiming staking rewards only check the default chain denom
                    vec![default_asset]
                },
            },
            Source::WhiteWhaleRewards {
                ..
            } => state.whitewhale.load(deps.storage)?.coins,

            Source::LiquidityAlliance {
                assets,
            } => todo!(),
        };
        Ok(from_assets)
    }
}
