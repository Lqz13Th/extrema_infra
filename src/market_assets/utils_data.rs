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
pub struct LeadtraderSubpositions {
    pub timestamp: u64,
    pub unique_code: String,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurrentLeadtrader {
    pub timestamp: u64,
    pub unique_code: String,
    pub nick_name: String,
    pub margin: f64,
    pub copy_pnl: f64,
    pub copy_amount: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PubLeadtraderInfo {
    pub data_version: u64,
    pub total_page: u64,
    pub pub_leadtraders: Vec<PubLeadtrader>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PubLeadtrader {
    pub unique_code: String,
    pub nick_name: String,
    pub aum: f64,
    pub copy_state: u64,
    pub copy_trader_num: u64,
    pub max_copy_trader_num: u64,
    pub accum_copy_trader_num: u64,
    pub lead_days: u64,
    pub win_ratio: f64,
    pub pnl_ratio: f64,
    pub pnl: f64,
}
