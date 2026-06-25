use serde::Deserialize;
use serde_json::Value;

use crate::arch::market_assets::{
    api_data::price_data::CandleData,
    api_general::{ts_to_micros, value_to_f64},
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestCandleHyperliquid {
    pub t: u64,
    pub s: String,
    pub o: Value,
    pub c: Value,
    pub h: Value,
    pub l: Value,
    #[serde(default)]
    pub v: Value,
}

impl RestCandleHyperliquid {
    pub fn into_candle_data(self, inst: &str) -> CandleData {
        CandleData {
            timestamp: ts_to_micros(self.t),
            inst: inst.to_string(),
            open: value_to_f64(&self.o),
            high: value_to_f64(&self.h),
            low: value_to_f64(&self.l),
            close: value_to_f64(&self.c),
            volume: value_to_f64(&self.v),
        }
    }
}
