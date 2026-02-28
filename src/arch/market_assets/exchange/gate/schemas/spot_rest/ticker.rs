use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::price_data::TickerData, api_general::get_micros_timestamp, base_data::InstrumentType,
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestTickerGateSpot {
    pub currency_pair: String,
    pub last: String,
}

impl From<RestTickerGateSpot> for TickerData {
    fn from(d: RestTickerGateSpot) -> Self {
        TickerData {
            // Gate spot ticker payload does not provide timestamp.
            timestamp: get_micros_timestamp(),
            inst: d.currency_pair,
            inst_type: InstrumentType::Spot,
            price: d.last.parse().unwrap_or_default(),
        }
    }
}
