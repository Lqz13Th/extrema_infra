use serde::Deserialize;
use serde_json::Value;

use crate::arch::market_assets::{
    api_data::price_data::OrderBookData,
    api_general::{ts_to_micros, value_to_f64},
};

#[derive(Clone, Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct RestOrderBookOkx {
    pub asks: Vec<Vec<Value>>,
    pub bids: Vec<Vec<Value>>,
    pub ts: String,
}

impl RestOrderBookOkx {
    pub fn into_orderbook_data(self, inst: &str) -> OrderBookData {
        OrderBookData {
            timestamp: ts_to_micros(self.ts.parse().unwrap_or_default()),
            inst: inst.to_string(),
            bids: levels_to_pairs(self.bids),
            asks: levels_to_pairs(self.asks),
        }
    }
}

fn levels_to_pairs(levels: Vec<Vec<Value>>) -> Vec<(f64, f64)> {
    levels
        .into_iter()
        .filter_map(|level| {
            let price = level.first().map(value_to_f64)?;
            let size = level.get(1).map(value_to_f64)?;
            Some((price, size))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_okx_orderbook_snapshot() {
        let raw = r#"{
            "asks": [["63901.0", "0.75", "0", "2"]],
            "bids": [["63900.0", "1.25", "0", "3"]],
            "ts": "1780540000123"
        }"#;

        let parsed: RestOrderBookOkx = serde_json::from_str(raw).unwrap();
        let book = parsed.into_orderbook_data("BTC_USDT_PERP");

        assert_eq!(book.timestamp, 1_780_540_000_123_000);
        assert_eq!(book.inst, "BTC_USDT_PERP");
        assert_eq!(book.bids, vec![(63900.0, 1.25)]);
        assert_eq!(book.asks, vec![(63901.0, 0.75)]);
    }
}
