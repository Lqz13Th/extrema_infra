use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::utils_data::FundingRateData,
    api_general::{get_micros_timestamp, ts_to_micros},
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestFundingRateGate {
    pub t: u64,
    pub r: String,
}

impl RestFundingRateGate {
    pub fn into_funding_rate_data(self, inst: &str) -> FundingRateData {
        FundingRateData {
            timestamp: get_micros_timestamp(),
            inst: inst.into(),
            funding_rate: self.r.parse().unwrap_or_default(),
            funding_time: ts_to_micros(self.t),
        }
    }
}
