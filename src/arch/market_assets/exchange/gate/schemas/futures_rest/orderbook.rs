use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::price_data::OrderBookData,
    api_general::{get_micros_timestamp, value_to_f64},
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestOrderBookGateFutures {
    #[serde(default)]
    pub current: Option<serde_json::Value>,
    #[serde(default)]
    pub asks: Vec<RestOrderBookLevelGateFutures>,
    #[serde(default)]
    pub bids: Vec<RestOrderBookLevelGateFutures>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum RestOrderBookLevelGateFutures {
    Object {
        p: serde_json::Value,
        s: serde_json::Value,
    },
    Array(Vec<serde_json::Value>),
}

impl RestOrderBookGateFutures {
    pub fn into_orderbook_data(self, inst: &str) -> OrderBookData {
        OrderBookData {
            timestamp: self
                .current
                .as_ref()
                .map(gate_current_to_micros)
                .unwrap_or_else(get_micros_timestamp),
            inst: inst.to_string(),
            bids: levels_to_pairs(self.bids),
            asks: levels_to_pairs(self.asks),
        }
    }
}

fn gate_current_to_micros(value: &serde_json::Value) -> u64 {
    if let Some(raw) = value.as_u64() {
        return raw.saturating_mul(1_000_000);
    }
    if let Some(raw) = value.as_f64() {
        return (raw * 1_000_000.0) as u64;
    }
    value
        .as_str()
        .and_then(|raw| raw.parse::<f64>().ok())
        .map(|raw| (raw * 1_000_000.0) as u64)
        .unwrap_or_else(get_micros_timestamp)
}

fn levels_to_pairs(levels: Vec<RestOrderBookLevelGateFutures>) -> Vec<(f64, f64)> {
    levels
        .into_iter()
        .filter_map(|level| match level {
            RestOrderBookLevelGateFutures::Object { p, s } => {
                Some((value_to_f64(&p), value_to_f64(&s)))
            },
            RestOrderBookLevelGateFutures::Array(values) => {
                let price = values.first().map(value_to_f64)?;
                let size = values.get(1).map(value_to_f64)?;
                Some((price, size))
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gate_futures_object_orderbook_snapshot() {
        let raw = r#"{
            "current": 1716803367.123,
            "asks": [{"p": "64321.5", "s": 10}],
            "bids": [{"p": "64320.9", "s": 8}]
        }"#;

        let parsed: RestOrderBookGateFutures = serde_json::from_str(raw).unwrap();
        let book = parsed.into_orderbook_data("BTC_USDT_PERP");

        assert_eq!(book.inst, "BTC_USDT_PERP");
        assert_eq!(book.bids, vec![(64320.9, 8.0)]);
        assert_eq!(book.asks, vec![(64321.5, 10.0)]);
    }

    #[test]
    fn parses_gate_futures_array_orderbook_snapshot() {
        let raw = r#"{
            "asks": [["64321.5", "10"]],
            "bids": [["64320.9", "8"]]
        }"#;

        let parsed: RestOrderBookGateFutures = serde_json::from_str(raw).unwrap();
        let book = parsed.into_orderbook_data("BTC_USDT_PERP");

        assert_eq!(book.bids, vec![(64320.9, 8.0)]);
        assert_eq!(book.asks, vec![(64321.5, 10.0)]);
    }
}
