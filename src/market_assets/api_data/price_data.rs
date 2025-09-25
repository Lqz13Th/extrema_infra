use serde::{Deserialize, Serialize};
use crate::market_assets::base_data::InstrumentType;

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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OrderBookData {
    pub timestamp: u64,
    pub inst: String,
    pub bids: Vec<(f64, f64)>, // (price, quantity)
    pub asks: Vec<(f64, f64)>,
}

