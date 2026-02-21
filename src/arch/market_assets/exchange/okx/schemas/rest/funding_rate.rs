use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::utils_data::{FundingRateData, FundingRateInfo},
    api_general::ts_to_micros,
    exchange::okx::api_utils::okx_inst_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestFundingRateOkx {
    pub instId: String,
    pub fundingRate: String,
    pub fundingTime: String,
    pub nextFundingTime: String,
    pub ts: String,
}

impl From<RestFundingRateOkx> for FundingRateData {
    fn from(d: RestFundingRateOkx) -> Self {
        FundingRateData {
            timestamp: ts_to_micros(d.ts.parse().unwrap_or_default()),
            inst: okx_inst_to_cli(&d.instId),
            funding_rate: d.fundingRate.parse().unwrap_or_default(),
            funding_time: ts_to_micros(d.fundingTime.parse().unwrap_or_default()),
        }
    }
}

impl From<RestFundingRateOkx> for FundingRateInfo {
    fn from(d: RestFundingRateOkx) -> Self {
        let funding_time_ms = d.fundingTime.parse::<u64>().unwrap_or_default();
        let next_funding_time_ms = d.nextFundingTime.parse::<u64>().unwrap_or_default();

        FundingRateInfo {
            timestamp: ts_to_micros(d.ts.parse().unwrap_or_default()),
            inst: okx_inst_to_cli(&d.instId),
            funding_interval_sec: next_funding_time_ms.saturating_sub(funding_time_ms) as f64
                / 1000.0,
        }
    }
}
