use serde::Deserialize;
use serde_json::Value;

use crate::arch::market_assets::{
    api_data::price_data::CandleData,
    api_general::{ts_to_micros, value_to_f64},
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestCandleGateFutures {
    pub t: u64,
    pub o: Value,
    pub h: Value,
    pub l: Value,
    pub c: Value,
    #[serde(default)]
    pub v: Value,
}

impl RestCandleGateFutures {
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
