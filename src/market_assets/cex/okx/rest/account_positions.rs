use serde::Deserialize;

use crate::market_assets::{
    api_general::ts_to_micros,
    account_data::PositionData,
    base_data::{InstrumentType, PositionSide},
    cex::okx::api_utils::okx_inst_to_cli,
};


#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestAccountPosOkx {
    pub instType: String,
    pub instId: String,
    pub posSide: String,
    pub pos: String,
    pub avgPx: String,
    pub markPx: String,
    pub margin: Option<String>,
    pub lever: String,
    pub uTime: String,
}

impl From<RestAccountPosOkx> for PositionData {
    fn from(d: RestAccountPosOkx) -> Self {
        PositionData {
            timestamp: ts_to_micros(d.uTime.parse().unwrap_or_default()),
            inst: okx_inst_to_cli(&d.instId),
            inst_type: match d.instType.as_str() {
                "SWAP" => InstrumentType::Perpetual,
                "FUTURES" => InstrumentType::Future,
                "SPOT" => InstrumentType::Spot,
                _ => InstrumentType::Unknown,
            },
            position_side: match d.posSide.to_uppercase().as_str() {
                "LONG" => PositionSide::Long,
                "SHORT" => PositionSide::Short,
                "NET" => {
                    let size = d.pos.parse::<f64>().unwrap_or(0.0);
                    if size >= 0.0 {
                        PositionSide::Long
                    } else {
                        PositionSide::Short
                    }
                }
                _ => PositionSide::Unknown,
            },
            size: d.pos.parse().unwrap_or_default(),
            avg_price: d.avgPx.parse().unwrap_or_default(),
            mark_price: d.markPx.parse().unwrap_or_default(),
            margin: d.margin.and_then(|m| m.parse::<f64>().ok()).unwrap_or(0.0),
            leverage: d.lever.parse().unwrap_or_default(),
        }
    }
}
