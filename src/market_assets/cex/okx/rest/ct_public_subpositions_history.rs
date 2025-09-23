use serde::{Deserialize, Serialize};

use crate::market_assets::{
    api_general::{get_micros_timestamp, ts_to_micros},
    base_data::{InstrumentType, MarginMode, PositionSide},
    utils_data::LeadtraderSubpositionHistory,
    cex::okx::api_utils::okx_inst_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RestSubPositionHistoryOkx {
    pub instId: String,
    pub subPosId: String,
    pub posSide: String,
    pub mgnMode: String,
    pub lever: String,
    pub openAvgPx: String,
    pub openTime: String,
    pub subPos: String,
    pub closeTime: Option<String>,
    pub closeAvgPx: Option<String>,
    pub pnl: Option<String>,
    pub pnlRatio: Option<String>,
    pub instType: String,
    pub margin: String,
    pub ccy: String,
    pub uniqueCode: String,
}

impl From<RestSubPositionHistoryOkx> for LeadtraderSubpositionHistory {
    fn from(d: RestSubPositionHistoryOkx) -> Self {
        let size_val = d.subPos.parse::<f64>().unwrap_or(0.0);

        LeadtraderSubpositionHistory {
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
                },
            },
            margin_mode: match d.mgnMode.to_lowercase().as_str() {
                "cross" => MarginMode::Cross,
                "isolated" => MarginMode::Isolated,
                _ => MarginMode::Cross,
            },
            ins_type: match d.instType.to_uppercase().as_str() {
                "SWAP" => InstrumentType::Perpetual,
                "SPOT" => InstrumentType::Spot,
                _ => InstrumentType::Spot,
            },
            leverage: d.lever.parse().unwrap_or_default(),
            size: size_val,
            margin: d.margin.parse().unwrap_or_default(),
            open_ts: ts_to_micros(d.openTime.parse().unwrap_or_default()),
            open_avg_price: d.openAvgPx.parse().unwrap_or_default(),
            close_ts: d
                .closeTime
                .map(|t| ts_to_micros(t.parse().unwrap_or_default()))
                .unwrap_or(0),
            close_avg_price: d
                .closeAvgPx
                .map(|p| p.parse().unwrap_or_default())
                .unwrap_or(0.0),
            pnl: d.pnl.map(|p| p.parse().unwrap_or_default()).unwrap_or(0.0),
            pnl_ratio: d
                .pnlRatio
                .map(|r| r.parse().unwrap_or_default())
                .unwrap_or(0.0),
        }
    }
}
