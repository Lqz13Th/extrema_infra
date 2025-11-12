use serde::{Deserialize, Serialize};
use crate::arch::market_assets::base_data::{
    InstrumentStatus, 
    InstrumentType, 
    MarginMode, 
    PositionSide,
};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InstrumentInfo {
    pub inst: String,
    pub inst_type: InstrumentType,
    pub lot_size: f64,
    pub tick_size: f64,
    pub min_lmt_size: f64,
    pub max_lmt_size: f64,
    pub min_mkt_size: f64,
    pub max_mkt_size: f64,
    pub contract_value: f64,
    pub contract_multiplier: f64,
    pub state: InstrumentStatus,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FundingRateData {
    pub timestamp: u64,
    pub inst: String,
    pub funding_rate: f64,
    pub funding_time: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FundingRateInfo {
    pub timestamp: u64,
    pub inst: String,
    pub funding_hours: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OpenInterest {
    pub timestamp: u64,
    pub inst: String,
    pub sum_open_interest: f64,
    pub sum_open_interest_value: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LeadtraderSubposition {
    pub timestamp: u64,
    pub unique_code: String,
    pub inst: String,
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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LeadtraderSubpositionHistory {
    pub timestamp: u64,
    pub unique_code: String,
    pub inst: String,
    pub subpos_id: String,
    pub pos_side: PositionSide,
    pub margin_mode: MarginMode,
    pub ins_type: InstrumentType,
    pub leverage: f64,
    pub size: f64,
    pub margin: f64,
    pub open_ts: u64,
    pub open_avg_price: f64,
    pub close_ts: u64,
    pub close_avg_price: f64,
    pub pnl: f64,
    pub pnl_ratio: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CurrentLeadtrader {
    pub timestamp: u64,
    pub unique_code: String,
    pub nick_name: String,
    pub margin: f64,
    pub copy_pnl: f64,
    pub copy_amount: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PubLeadtraderInfo {
    pub data_version: u64,
    pub total_page: u64,
    pub pub_leadtraders: Vec<PubLeadtrader>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PubLeadtraderStats {
    pub timestamp: u64,
    pub win_ratio: f64,
    pub profit_days: u64,
    pub loss_days: f64,
    pub invest_amount: f64,
    pub avg_sub_pos_national: f64,
    pub current_copy_trader_pnl: f64,
}
