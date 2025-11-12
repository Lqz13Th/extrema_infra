use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::utils_data::LeadtraderSubposition,
    exchange::okx::api_utils::okx_inst_to_cli,
    api_general::{
        get_micros_timestamp, 
        ts_to_micros,
    },
    base_data::{
        InstrumentType, 
        MarginMode, 
        PositionSide,
    },
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestSubPositionOkx {
    pub instId: String,
    pub subPosId: String,
    pub posSide: String,
    pub mgnMode: String,
    pub lever: String,
    pub openAvgPx: String,
    pub openTime: String,
    pub subPos: String,
    pub instType: String,
    pub margin: String,
    pub upl: String,
    pub uplRatio: String,
    pub markPx: Option<String>,
    pub uniqueCode: String,
    pub ccy: String,
}

impl From<RestSubPositionOkx> for LeadtraderSubposition {
    fn from(d: RestSubPositionOkx) -> Self {
        let size_val = d.subPos.parse::<f64>().unwrap_or(0.0);
        
        LeadtraderSubposition {
            timestamp: get_micros_timestamp(),
            unique_code: d.uniqueCode,
            inst: okx_inst_to_cli(&d.instId),
            subpos_id: d.subPosId,
            pos_side: match d.posSide.to_lowercase().as_str() {
                "long" => PositionSide::Long,
                "short" => PositionSide::Short,
                _ => {
                    if size_val >= 0.0 {
                        PositionSide::Long
                    } else {
                        PositionSide::Short
                    }
                }
            },
            margin_mode: match d.mgnMode.to_lowercase().as_str() {
                "cross" => MarginMode::Cross,
                "isolated" => MarginMode::Isolated,
                _ => MarginMode::Cross,
            },
            leverage: d.lever.parse().unwrap_or_default(),
            open_ts: ts_to_micros(d.openTime.parse().unwrap_or_default()),
            open_avg_price: d.openAvgPx.parse().unwrap_or_default(),
            size: size_val,
            ins_type: match d.instType.to_uppercase().as_str() { 
                "SWAP" => InstrumentType::Perpetual,
                "SPOT" => InstrumentType::Spot,
                _ => InstrumentType::Spot,
            },
            margin: d.margin.parse().unwrap_or_default(),
        }
    }
}
