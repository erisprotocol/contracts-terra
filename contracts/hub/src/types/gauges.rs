use cosmwasm_std::{Addr, QuerierWrapper, StdResult};
use eris::amp_gauges::{get_amp_tune_info, get_amp_validator_infos, GaugeInfoResponse as AmpGauge};
use eris::emp_gauges::{get_emp_tune_info, get_emp_validator_infos, GaugeInfoResponse as EmpGauge};
use eris::governance_helper::get_s_from_period;
use itertools::Itertools;

pub trait GaugeLoader {
    fn get_amp_tune_info(&self, querier: &QuerierWrapper, amp_gauges: Addr) -> StdResult<AmpGauge>;
    fn get_emp_tune_info(&self, querier: &QuerierWrapper, emp_gauges: Addr) -> StdResult<EmpGauge>;
}

/// This loader is used for tuning delegations. It loads the gauges from the TuneInfo storage of each contract.
pub struct TuneInfoGaugeLoader {}
impl GaugeLoader for TuneInfoGaugeLoader {
    fn get_amp_tune_info(&self, querier: &QuerierWrapper, amp_gauges: Addr) -> StdResult<AmpGauge> {
        get_amp_tune_info(querier, amp_gauges)
    }
    fn get_emp_tune_info(&self, querier: &QuerierWrapper, emp_gauges: Addr) -> StdResult<EmpGauge> {
        get_emp_tune_info(querier, emp_gauges)
    }
}

/// This loader is only used to simulate delegation queries at any period
pub struct PeriodGaugeLoader {
    pub period: u64,
}
impl GaugeLoader for PeriodGaugeLoader {
    fn get_amp_tune_info(&self, querier: &QuerierWrapper, amp_gauges: Addr) -> StdResult<AmpGauge> {
        let infos = get_amp_validator_infos(querier, amp_gauges, self.period)?;

        Ok(AmpGauge {
            tune_ts: get_s_from_period(self.period),
            vamp_points: infos
                .into_iter()
                .map(|(val, info)| (val, info.fixed_amount + info.voting_power))
                .sorted_by(|(_, a), (_, b)| b.cmp(a)) // Sort in descending order
                .collect_vec(),
        })
    }

    fn get_emp_tune_info(&self, querier: &QuerierWrapper, emp_gauges: Addr) -> StdResult<EmpGauge> {
        let infos = get_emp_validator_infos(querier, emp_gauges, self.period)?;

        Ok(EmpGauge {
            tune_ts: get_s_from_period(self.period),
            tune_period: self.period,
            emp_points: infos
                .into_iter()
                .map(|(val, info)| (val, info.fixed_amount + info.voting_power))
                .sorted_by(|(_, a), (_, b)| b.cmp(a)) // Sort in descending order
                .collect_vec(),
        })
    }
}
