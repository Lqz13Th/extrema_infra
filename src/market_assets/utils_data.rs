use serde::{Deserialize, Serialize};

use super::base_data::{InstrumentType, MarginMode, PositionSide};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketInfoData {
    pub symbol: String,
    pub min_order_size: f64,
    pub max_order_size: f64,
    pub price_precision: u32,
    pub lot_size: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PubCopytraderSubpositions {
    pub timestamp: u64,
    pub copytrader_id: String,
    pub symbol: String,
    pub subpos_id: String,
    pub pos_side: PositionSide,
    pub margin_mode: MarginMode,
    pub leverage: f64,
    pub open_ts: u64,
    pub open_avg_price: f64,
    pub size: f64,
    pub ins_type: InstrumentType,
    pub margin: f64,
}