use serde::{Deserialize, Serialize};

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
    pub side: String, // "LONG" / "SHORT"
    pub size: f64,
    pub entry_price: f64,
    pub unrealized_pnl: f64,
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
