use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceData {
    pub asset: String,
    pub free: f64,
    pub locked: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionData {
    pub symbol: String,
    pub side: String, // "LONG" / "SHORT"
    pub size: f64,
    pub entry_price: f64,
    pub unrealized_pnl: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderData {
    pub order_id: String,
    pub symbol: String,
    pub side: String,
    pub price: f64,
    pub quantity: f64,
    pub status: String, // "NEW", "FILLED", ...
}
