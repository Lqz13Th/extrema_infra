use serde::{Deserialize, Serialize};
use crate::arch::market_assets::base_data::InstrumentType;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TickerData {
    pub timestamp: u64,
    pub inst: String,
    pub inst_type: InstrumentType,
    pub price: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CandleData {
    pub timestamp: u64,
    pub inst: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl CandleData {
    pub fn new(inst: &str, timestamp: u64, open: f64, high: f64, low: f64, close: f64) -> Self {
        Self {
            timestamp,
            inst: inst.to_string(),
            open,
            high,
            low,
            close,
            volume: 0.0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OrderBookData {
    pub timestamp: u64,
    pub inst: String,
    pub bids: Vec<(f64, f64)>, // (price, quantity)
    pub asks: Vec<(f64, f64)>,
}

