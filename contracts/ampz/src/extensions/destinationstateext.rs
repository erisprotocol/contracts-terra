use astroport::asset::AssetInfo;
use eris::ampz::{DestinationRuntime, DestinationState};

pub trait DestinationStateExt {
    fn to_runtime(self, asset_infos: Vec<AssetInfo>) -> DestinationRuntime;
}

impl DestinationStateExt for DestinationState {
    fn to_runtime(self, asset_infos: Vec<AssetInfo>) -> DestinationRuntime {
        match self {
            DestinationState::DepositAmplifier {
                receiver,
            } => DestinationRuntime::DepositAmplifier {
                receiver,
            },
            DestinationState::DepositArbVault {
                receiver,
            } => DestinationRuntime::DepositArbVault {
                receiver,
            },
            DestinationState::DepositFarm {
                farm,
                receiver,
            } => DestinationRuntime::DepositFarm {
                asset_infos,
                farm,
                receiver,
            },
            DestinationState::DepositLiquidity {
                lp_token,
                dex,
            } => DestinationRuntime::DepositLiquidity {
                asset_infos,
                lp_token,
                dex,
            },
            DestinationState::SwapTo {
                asset_info,
                receiver,
            } => DestinationRuntime::SendSwapResultToUser {
                asset_info,
                receiver,
            },
            DestinationState::Repay {
                market,
            } => DestinationRuntime::Repay {
                market,
            },
            DestinationState::DepositCollateral {
                market,
            } => DestinationRuntime::DepositCollateral {
                market,
            },
            DestinationState::DepositTAmplifier {
                receiver,
                asset_info,
            } => DestinationRuntime::DepositTAmplifier {
                receiver,
                asset_info,
            },
            DestinationState::ExecuteContract {
                asset_info,
                contract,
                msg,
            } => DestinationRuntime::ExecuteContract {
                contract,
                msg,
                asset_info,
            },
            DestinationState::LiquidityAlliance {
                gauge,
                lp_info,
                compounding,
            } => DestinationRuntime::LiquidityAlliance {
                asset_infos,
                gauge,
                lp_info,
                compounding,
            },
        }
    }
}
