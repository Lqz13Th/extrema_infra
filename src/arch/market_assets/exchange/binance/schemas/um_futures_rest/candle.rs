use serde::Deserialize;
use serde_json::Value;

use crate::arch::market_assets::{
    api_data::price_data::CandleData,
    api_general::{ts_to_micros, value_to_f64},
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestCandleBinanceUM(pub Vec<Value>);

impl RestCandleBinanceUM {
    pub fn into_candle_data(self, inst: &str) -> CandleData {
        let values = self.0;
        CandleData {
            timestamp: ts_to_micros(values.first().and_then(Value::as_u64).unwrap_or_default()),
            inst: inst.to_string(),
            open: values.get(1).map(value_to_f64).unwrap_or_default(),
            high: values.get(2).map(value_to_f64).unwrap_or_default(),
            low: values.get(3).map(value_to_f64).unwrap_or_default(),
            close: values.get(4).map(value_to_f64).unwrap_or_default(),
            volume: values.get(5).map(value_to_f64).unwrap_or_default(),
        }
    }
}
