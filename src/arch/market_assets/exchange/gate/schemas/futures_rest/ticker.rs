use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::price_data::{MarkPriceData, TickerData},
    api_general::get_micros_timestamp,
    base_data::InstrumentType,
    exchange::gate::api_utils::gate_fut_inst_to_cli,
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestTickerGateFutures {
    pub contract: String, // BTC_USDT
    pub last: String,
    #[serde(default)]
    pub mark_price: String,
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

impl From<RestTickerGateFutures> for MarkPriceData {
    fn from(d: RestTickerGateFutures) -> Self {
        MarkPriceData {
            timestamp: get_micros_timestamp(),
            inst: gate_fut_inst_to_cli(&d.contract),
            inst_type: InstrumentType::Perpetual,
            mark_price: d.mark_price.parse().unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn converts_ticker_mark_price() {
        let raw: RestTickerGateFutures = serde_json::from_value(json!({
            "contract": "BTC_USDT",
            "last": "64120.1",
            "mark_price": "64123.4"
        }))
        .unwrap();

        let data = MarkPriceData::from(raw);

        assert_eq!(data.inst, "BTC_USDT_PERP");
        assert_eq!(data.inst_type, InstrumentType::Perpetual);
        assert_eq!(data.mark_price, 64123.4);
    }

    #[test]
    fn parses_ticker_without_mark_price() {
        let raw: RestTickerGateFutures = serde_json::from_value(json!({
            "contract": "BTC_USDT",
            "last": "64120.1"
        }))
        .unwrap();

        assert_eq!(TickerData::from(raw.clone()).price, 64120.1);
        assert_eq!(MarkPriceData::from(raw).mark_price, 0.0);
    }
}
