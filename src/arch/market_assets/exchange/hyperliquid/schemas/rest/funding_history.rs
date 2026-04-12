use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::utils_data::FundingRateData,
    api_general::{
        de_string_from_any, de_u64_from_string_or_number, get_micros_timestamp, ts_to_micros,
    },
    exchange::hyperliquid::api_utils::hyperliquid_perp_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestFundingHistoryHyperliquid {
    #[serde(deserialize_with = "de_string_from_any")]
    pub coin: String,
    #[serde(deserialize_with = "de_string_from_any")]
    pub fundingRate: String,
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub premium: String,
    #[serde(deserialize_with = "de_u64_from_string_or_number")]
    pub time: u64,
}

impl From<RestFundingHistoryHyperliquid> for FundingRateData {
    fn from(d: RestFundingHistoryHyperliquid) -> Self {
        FundingRateData {
            timestamp: get_micros_timestamp(),
            inst: hyperliquid_perp_to_cli(&d.coin),
            funding_rate: d.fundingRate.parse().unwrap_or_default(),
            funding_time: ts_to_micros(d.time),
        }
    }
}
