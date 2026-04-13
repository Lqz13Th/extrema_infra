use serde::Deserialize;
use serde_json::Value;

use crate::arch::{
    market_assets::{
        api_general::value_to_f64,
        base_data::{InstrumentType, MarginMode, PositionSide},
        exchange::hyperliquid::api_utils::hyperliquid_inst_to_cli,
    },
    strategy_base::handler::lob_events::WsAccPosition,
    traits::conversion::IntoWsData,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsAccountPositionHyperliquid {
    #[serde(rename = "type")]
    kind: String,
    position: WsPositionHyperliquid,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct WsPositionHyperliquid {
    coin: String,
    #[serde(default)]
    szi: Value,
    #[serde(default)]
    entryPx: Value,
    leverage: WsLeverageHyperliquid,
}

#[derive(Clone, Debug, Deserialize)]
struct WsLeverageHyperliquid {
    #[serde(rename = "type")]
    kind: String,
    value: Value,
}

impl IntoWsData for WsAccountPositionHyperliquid {
    type Output = WsAccPosition;

    fn into_ws(self) -> Self::Output {
        let size = value_to_f64(&self.position.szi);

        WsAccPosition {
            inst: hyperliquid_inst_to_cli(&self.position.coin),
            inst_type: if self.position.coin.contains('/') || self.position.coin.starts_with('@') {
                InstrumentType::Spot
            } else {
                InstrumentType::Perpetual
            },
            avg_price: value_to_f64(&self.position.entryPx),
            size,
            position_side: if size > 0.0 {
                PositionSide::Long
            } else if size < 0.0 {
                PositionSide::Short
            } else {
                PositionSide::Both
            },
            margin_mode: match self.position.leverage.kind.to_ascii_lowercase().as_str() {
                "cross" => MarginMode::Cross,
                "isolated" => MarginMode::Isolated,
                _ => MarginMode::Unknown,
            },
        }
    }
}
