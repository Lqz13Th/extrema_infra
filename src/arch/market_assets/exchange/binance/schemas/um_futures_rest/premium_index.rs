use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::utils_data::FundingRateData, api_general::ts_to_micros,
    exchange::binance::api_utils::binance_inst_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestPremiumIndexBinanceUM {
    pub symbol: String,
    pub markPrice: String,
    pub indexPrice: String,
    pub estimatedSettlePrice: String,
    pub lastFundingRate: String,
    pub interestRate: String,
    pub nextFundingTime: u64,
    pub time: u64,
}

impl From<RestPremiumIndexBinanceUM> for FundingRateData {
    fn from(d: RestPremiumIndexBinanceUM) -> Self {
        FundingRateData {
            timestamp: ts_to_micros(d.time),
            inst: binance_inst_to_cli(&d.symbol),
            funding_rate: d.lastFundingRate.parse().unwrap_or_default(),
            funding_time: ts_to_micros(d.nextFundingTime),
        }
    }
}
