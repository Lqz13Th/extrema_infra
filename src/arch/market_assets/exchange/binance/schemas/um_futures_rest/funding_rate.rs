use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::utils_data::FundingRateData,
    api_general::{ts_to_micros, get_micros_timestamp},
    exchange::binance::api_utils::binance_inst_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestFundingRateBinanceUM {
    pub symbol: String,
    pub fundingRate: String,
    pub fundingTime: u64,
    pub markPrice: Option<String>,
}


impl From<RestFundingRateBinanceUM> for FundingRateData {
    fn from(d: RestFundingRateBinanceUM) -> Self {
        FundingRateData {
            timestamp: get_micros_timestamp(),
            inst: binance_inst_to_cli(&d.symbol),
            funding_rate: d.fundingRate.parse::<f64>().unwrap_or_default(),
            funding_time: ts_to_micros(d.fundingTime),
        }
    }
}
