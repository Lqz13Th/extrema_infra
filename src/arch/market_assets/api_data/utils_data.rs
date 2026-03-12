use serde::{Deserialize, Serialize};

use crate::arch::market_assets::base_data::{InstrumentStatus, InstrumentType};
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InstrumentInfo {
    pub inst: String,
    pub inst_code: Option<String>,
    pub inst_type: InstrumentType,
    pub lot_size: f64,
    pub tick_size: f64,
    pub min_lmt_size: f64,
    pub max_lmt_size: f64,
    pub min_mkt_size: f64,
    pub max_mkt_size: f64,
    pub min_notional: Option<f64>,
    pub contract_value: Option<f64>,
    pub contract_multiplier: Option<f64>,
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
    pub funding_interval_sec: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OpenInterest {
    pub timestamp: u64,
    pub inst: String,
    pub sum_open_interest: f64,
    pub sum_open_interest_value: Option<f64>,
}
