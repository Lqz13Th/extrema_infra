use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::utils_data::FundingRateData,
    api_general::{get_micros_timestamp, ts_to_micros},
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestFundingRateGateFutures {
    pub t: u64,
    pub r: String,
}

impl From<(RestFundingRateGateFutures, &str)> for FundingRateData {
    fn from((data, inst): (RestFundingRateGateFutures, &str)) -> Self {
        FundingRateData {
            timestamp: get_micros_timestamp(),
            inst: inst.into(),
            funding_rate: data.r.parse().unwrap_or_default(),
            funding_time: ts_to_micros(data.t),
        }
    }
}
