use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::utils_data::FundingRateData,
    api_general::{get_micros_timestamp, ts_to_micros},
    exchange::gate::api_utils::gate_inst_to_cli,
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestContractGate {
    pub name: String,
    #[serde(default)]
    pub funding_rate: String,
    #[serde(default)]
    pub funding_next_apply: Option<u64>,
}

impl RestContractGate {
    pub fn into_funding_rate_data(self) -> FundingRateData {
        FundingRateData {
            timestamp: get_micros_timestamp(),
            inst: gate_inst_to_cli(&self.name),
            funding_rate: self.funding_rate.parse().unwrap_or_default(),
            funding_time: ts_to_micros(self.funding_next_apply.unwrap_or_default()),
        }
    }
}
