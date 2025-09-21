use serde::{Deserialize, Serialize};
use crate::market_assets::base_data::{InstrumentType, PositionSide};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BalanceData {
    pub timestamp: u64,
    pub asset: String,
    pub total: f64,
    pub frozen: f64,
    pub available: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PositionData {
    pub timestamp: u64,
    pub inst: String,
    pub inst_type: InstrumentType,
    pub position_side: PositionSide, // "LONG" / "SHORT"
    pub size: f64,
    pub avg_price: f64,
    pub mark_price: f64,
    pub margin: f64,
    pub leverage: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderData {
    pub timestamp: u64,
    pub order_id: String,
    pub inst: String,
    pub side: String,
    pub price: f64,
    pub quantity: f64,
    pub status: String, // "NEW", "FILLED", ...
}
