use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::price_data::TickerData, api_general::ts_to_micros, base_data::InstrumentType,
    exchange::binance::api_utils::binance_fut_inst_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestTickerBinanceUM {
    pub symbol: String,
    pub price: String,
    pub time: u64,
}

impl From<RestTickerBinanceUM> for TickerData {
    fn from(d: RestTickerBinanceUM) -> Self {
        let inst_type = if d.symbol.contains('_') {
            InstrumentType::Futures
        } else {
            InstrumentType::Perpetual
        };

        TickerData {
            timestamp: ts_to_micros(d.time),
            inst: binance_fut_inst_to_cli(&d.symbol),
            inst_type,
            price: d.price.parse().unwrap_or_default(),
        }
    }
}
