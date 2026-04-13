use serde::Deserialize;

use crate::arch::{
    market_assets::{
        base_data::{InstrumentType, MarginMode, PositionSide},
        exchange::binance::api_utils::binance_fut_inst_to_cli,
    },
    strategy_base::handler::lob_events::WsAccPosition,
    traits::conversion::IntoWsData,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsAccountPositionBinanceUM {
    s: String,  // Symbol
    pa: String, // Position amount
    ep: String, // Entry price
    mt: String, // Margin type
    ps: String, // Position side ("BOTH", "LONG", "SHORT")
}

impl IntoWsData for WsAccountPositionBinanceUM {
    type Output = WsAccPosition;

    fn into_ws(self) -> WsAccPosition {
        WsAccPosition {
            inst: binance_fut_inst_to_cli(&self.s),
            inst_type: if self.s.contains('_') {
                InstrumentType::Futures
            } else {
                InstrumentType::Perpetual
            },
            size: self.pa.parse().unwrap_or_default(),
            avg_price: self.ep.parse().unwrap_or_default(),
            position_side: match self.ps.as_str() {
                "LONG" => PositionSide::Long,
                "SHORT" => PositionSide::Short,
                "BOTH" => PositionSide::Both,
                _ => PositionSide::Unknown,
            },
            margin_mode: match self.mt.to_lowercase().as_str() {
                "cross" => MarginMode::Cross,
                "isolated" => MarginMode::Isolated,
                _ => MarginMode::Unknown,
            },
        }
    }
}
