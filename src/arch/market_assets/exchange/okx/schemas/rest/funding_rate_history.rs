use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::utils_data::FundingRateData,
    api_general::{get_micros_timestamp, ts_to_micros},
    exchange::okx::api_utils::okx_inst_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestFundingRateHistoryOkx {
    pub instId: String,
    pub fundingRate: String,
    pub fundingTime: String,
}

impl From<RestFundingRateHistoryOkx> for FundingRateData {
    fn from(d: RestFundingRateHistoryOkx) -> Self {
        FundingRateData {
            timestamp: get_micros_timestamp(),
            inst: okx_inst_to_cli(&d.instId),
            funding_rate: d.fundingRate.parse().unwrap_or_default(),
            funding_time: ts_to_micros(d.fundingTime.parse().unwrap_or_default()),
        }
    }
}
