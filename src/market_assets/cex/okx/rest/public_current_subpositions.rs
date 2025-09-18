use serde::{Deserialize, Serialize};

use crate::market_assets::{
    api_general::{get_micros_timestamp, ts_to_micros},
    base_data::{InstrumentType, MarginMode, PositionSide},
    utils_data::LeadtraderSubpositions,
    cex::okx::api_utils::okx_swap_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize, Serialize)]
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

impl From<RestSubPositionOkx> for LeadtraderSubpositions {
    fn from(d: RestSubPositionOkx) -> Self {
        let size_val = d.subPos.parse::<f64>().unwrap_or(0.0);

        let pos_side = match d.posSide.to_lowercase().as_str() {
            "long" => PositionSide::Long,
            "short" => PositionSide::Short,
            _ => {
                if size_val >= 0.0 {
                    PositionSide::Long
                } else {
                    PositionSide::Short
                }
            }
        };

        let margin_mode = match d.mgnMode.to_lowercase().as_str() {
            "cross" => MarginMode::Cross,
            "isolated" => MarginMode::Isolated,
            _ => MarginMode::Cross,
        };

        let ins_type = match d.instType.to_uppercase().as_str() {
            "SWAP" => InstrumentType::Perpetual,
            "SPOT" => InstrumentType::Spot,
            _ => InstrumentType::Spot,
        };

        LeadtraderSubpositions {
            timestamp: get_micros_timestamp(),
            unique_code: d.uniqueCode,
            inst: okx_swap_to_cli(&d.instId),
            subpos_id: d.subPosId,
            pos_side,
            margin_mode,
            leverage: d.lever.parse::<f64>().unwrap_or(1.0),
            open_ts: d.openTime.parse::<u64>().map(ts_to_micros).unwrap_or(0),
            open_avg_price: d.openAvgPx.parse::<f64>().unwrap_or(0.0),
            size: size_val,
            ins_type,
            margin: d.margin.parse::<f64>().unwrap_or(0.0),
        }
    }
}
