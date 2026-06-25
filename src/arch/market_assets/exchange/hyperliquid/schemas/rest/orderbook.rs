use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::price_data::OrderBookData,
    api_general::{ts_to_micros, value_to_f64},
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestOrderBookHyperliquid {
    pub levels: Vec<Vec<RestOrderBookLevelHyperliquid>>,
    pub time: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RestOrderBookLevelHyperliquid {
    pub px: serde_json::Value,
    pub sz: serde_json::Value,
}

impl RestOrderBookHyperliquid {
    pub fn into_orderbook_data(self, inst: &str, depth: usize) -> OrderBookData {
        let mut levels = self.levels.into_iter();
        let bids = levels.next().unwrap_or_default();
        let asks = levels.next().unwrap_or_default();

        OrderBookData {
            timestamp: ts_to_micros(self.time),
            inst: inst.to_string(),
            bids: levels_to_pairs(bids, depth),
            asks: levels_to_pairs(asks, depth),
        }
    }
}

fn levels_to_pairs(levels: Vec<RestOrderBookLevelHyperliquid>, depth: usize) -> Vec<(f64, f64)> {
    let iter = levels
        .into_iter()
        .map(|level| (value_to_f64(&level.px), value_to_f64(&level.sz)));
    if depth == 0 {
        iter.collect()
    } else {
        iter.take(depth).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hyperliquid_orderbook_snapshot() {
        let raw = r#"{
            "time": 1780540000123,
            "levels": [
                [{"px": "63900.0", "sz": "1.25", "n": 3}],
                [{"px": "63901.0", "sz": "0.75", "n": 2}]
            ]
        }"#;

        let parsed: RestOrderBookHyperliquid = serde_json::from_str(raw).unwrap();
        let book = parsed.into_orderbook_data("BTC_USDC_PERP", 1);

        assert_eq!(book.timestamp, 1_780_540_000_123_000);
        assert_eq!(book.inst, "BTC_USDC_PERP");
        assert_eq!(book.bids, vec![(63900.0, 1.25)]);
        assert_eq!(book.asks, vec![(63901.0, 0.75)]);
    }
}
