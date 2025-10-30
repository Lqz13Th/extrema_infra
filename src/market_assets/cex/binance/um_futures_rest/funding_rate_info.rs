use serde::Deserialize;

use crate::market_assets::{
    api_data::utils_data::FundingRateInfo,
    api_general::get_micros_timestamp,
    cex::binance::api_utils::binance_inst_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestFundingInfoBinanceUM {
    pub symbol: String,
    pub adjustedFundingRateCap: String,
    pub adjustedFundingRateFloor: String,
    pub fundingIntervalHours: u64,
    pub disclaimer: Option<bool>,
}



impl From<RestFundingInfoBinanceUM> for FundingRateInfo {
    fn from(d: RestFundingInfoBinanceUM) -> Self {
        FundingRateInfo {
            timestamp: get_micros_timestamp(),
            inst: binance_inst_to_cli(&d.symbol),
            funding_hours: d.fundingIntervalHours as f64,
        }
    }
}
