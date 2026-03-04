use serde::Deserialize;

use crate::arch::{
    market_assets::{
        base_data::{InstrumentType, MarginMode, PositionSide},
        exchange::okx::api_utils::okx_inst_to_cli,
    },
    strategy_base::handler::lob_events::WsAccPosition,
    traits::conversion::IntoWsData,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsAccountPositionOkx {
    instId: String,
    instType: String,
    mgnMode: String,
    posSide: String,
    pos: String,
    avgPx: String,
}

impl IntoWsData for WsAccountPositionOkx {
    type Output = WsAccPosition;
    fn into_ws(self) -> WsAccPosition {
        WsAccPosition {
            inst: okx_inst_to_cli(&self.instId),
            inst_type: match self.instType.as_str() {
                "SWAP" => InstrumentType::Perpetual,
                "FUTURES" => InstrumentType::Futures,
                "SPOT" => InstrumentType::Spot,
                "OPTIONS" => InstrumentType::Options,
                _ => InstrumentType::Unknown,
            },
            size: self.pos.parse().unwrap_or_default(),
            avg_price: self.avgPx.parse().unwrap_or_default(),
            position_side: match self.posSide.as_str() {
                "long" => PositionSide::Long,
                "short" => PositionSide::Short,
                "net" => PositionSide::Both,
                _ => PositionSide::Unknown,
            },
            margin_mode: match self.mgnMode.to_lowercase().as_str() {
                "cross" => MarginMode::Cross,
                "isolated" => MarginMode::Isolated,
                _ => MarginMode::Unknown,
            },
        }
    }
}
