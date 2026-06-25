use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::price_data::OrderBookData,
    api_general::{get_micros_timestamp, value_to_f64},
};

#[derive(Clone, Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct RestOrderBookBinanceUM {
    pub lastUpdateId: u64,
    pub bids: Vec<[serde_json::Value; 2]>,
    pub asks: Vec<[serde_json::Value; 2]>,
}

impl RestOrderBookBinanceUM {
    pub fn into_orderbook_data(self, inst: &str) -> OrderBookData {
        OrderBookData {
            timestamp: get_micros_timestamp(),
            inst: inst.to_string(),
            bids: levels_to_pairs(self.bids),
            asks: levels_to_pairs(self.asks),
        }
    }
}

fn levels_to_pairs(levels: Vec<[serde_json::Value; 2]>) -> Vec<(f64, f64)> {
    levels
        .into_iter()
        .map(|[price, size]| (value_to_f64(&price), value_to_f64(&size)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_binance_um_orderbook_snapshot() {
        let raw = r#"{
            "lastUpdateId": 1027024,
            "E": 1589436922972,
            "T": 1589436922959,
            "bids": [["4.00000000", "431.00000000"]],
            "asks": [["4.00000200", "12.00000000"]]
        }"#;

        let parsed: RestOrderBookBinanceUM = serde_json::from_str(raw).unwrap();
        let book = parsed.into_orderbook_data("BTC_USDT_PERP");

        assert_eq!(book.inst, "BTC_USDT_PERP");
        assert_eq!(book.bids, vec![(4.0, 431.0)]);
        assert_eq!(book.asks, vec![(4.000002, 12.0)]);
    }
}
