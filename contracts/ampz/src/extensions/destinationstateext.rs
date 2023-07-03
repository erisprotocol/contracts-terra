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
        }
    }
}
