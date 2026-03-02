use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::price_data::TickerData, api_general::get_micros_timestamp, base_data::InstrumentType,
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestTickerBinanceSpot {
    pub symbol: String,
    pub price: String,
}

impl From<RestTickerBinanceSpot> for TickerData {
    fn from(d: RestTickerBinanceSpot) -> Self {
        TickerData {
            timestamp: get_micros_timestamp(),
            inst: d.symbol,
            inst_type: InstrumentType::Spot,
            price: d.price.parse().unwrap_or_default(),
        }
    }
}
