use serde::Deserialize;

use crate::market_assets::{
    api_data::utils_data::OpenInterest,
    api_general::ts_to_micros,
    cex::binance::api_utils::binance_inst_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestOpenInterestBinanceUM {
    pub symbol: String,
    pub sumOpenInterest: String,
    pub sumOpenInterestValue: String,
    pub CMCCirculatingSupply: Option<String>,
    pub timestamp: String,
}


impl From<RestOpenInterestBinanceUM> for OpenInterest {
    fn from(d: RestOpenInterestBinanceUM) -> Self {
        OpenInterest {
            timestamp: ts_to_micros(d.timestamp.parse::<u64>().unwrap_or_default()),
            inst: binance_inst_to_cli(&d.symbol),
            sum_open_interest: d.sumOpenInterest.parse::<f64>().unwrap_or_default(),
            sum_open_interest_value: Some(d
                .sumOpenInterestValue
                .parse::<f64>()
                .unwrap_or_default()
            ),
        }
    }
}
