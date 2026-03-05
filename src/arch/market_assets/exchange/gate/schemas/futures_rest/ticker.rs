use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::price_data::TickerData, api_general::get_micros_timestamp, base_data::InstrumentType,
    exchange::gate::api_utils::gate_fut_inst_to_cli,
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestTickerGateFutures {
    pub contract: String, // BTC_USDT
    pub last: String,
}

impl From<RestTickerGateFutures> for TickerData {
    fn from(d: RestTickerGateFutures) -> Self {
        TickerData {
            timestamp: get_micros_timestamp(),
            inst: gate_fut_inst_to_cli(&d.contract),
            inst_type: InstrumentType::Perpetual,
            price: d.last.parse().unwrap_or_default(),
        }
    }
}
