use serde::Deserialize;

use crate::market_assets::{
    api_data::price_data::TickerData,
    cex::prelude::okx_inst_to_cli,
    api_general::ts_to_micros,
    base_data::InstrumentType,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestMarketTickerOkx {
    pub instId: String,
    pub last: String,
    pub ts: String,
    pub instType: String,
}

impl From<RestMarketTickerOkx> for TickerData {
    fn from(d: RestMarketTickerOkx) -> Self {
        TickerData {
            timestamp: ts_to_micros(d.ts.parse().unwrap_or_default()),
            inst: okx_inst_to_cli(&d.instId),
            inst_type: match d.instType.as_str() {
                "SPOT" => InstrumentType::Spot,
                "FUTURES" => InstrumentType::Futures,
                "SWAP" => InstrumentType::Perpetual,
                _ => InstrumentType::Unknown,
            },
            price: d.last.parse().unwrap_or_default(),
        }
    }
}
