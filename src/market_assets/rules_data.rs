use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketInfoData {
    pub symbol: String,
    pub min_order_size: f64,
    pub max_order_size: f64,
    pub price_precision: u32,
    pub lot_size: f64,
}