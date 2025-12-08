use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::utils_data::OpenInterest, api_general::ts_to_micros,
    exchange::binance::api_utils::binance_inst_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestOpenInterestBinanceCM {
    pub pair: String,
    pub sumOpenInterest: String,
    pub sumOpenInterestValue: String,
    pub timestamp: u64,
}

impl From<RestOpenInterestBinanceCM> for OpenInterest {
    fn from(d: RestOpenInterestBinanceCM) -> Self {
        OpenInterest {
            timestamp: ts_to_micros(d.timestamp),
            inst: binance_inst_to_cli(&d.pair),
            sum_open_interest: d.sumOpenInterest.parse().unwrap_or_default(),
            sum_open_interest_value: Some(d.sumOpenInterestValue.parse().unwrap_or_default()),
        }
    }
}
