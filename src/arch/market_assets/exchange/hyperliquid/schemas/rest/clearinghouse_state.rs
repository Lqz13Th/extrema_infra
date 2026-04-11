use serde::Deserialize;
use serde_json::Value;

use crate::arch::market_assets::{
    api_data::account_data::PositionData,
    api_general::value_to_f64,
    base_data::{InstrumentType, MarginMode, PositionSide},
    exchange::hyperliquid::api_utils::hyperliquid_perp_to_cli,
};
use crate::arch::strategy_base::handler::lob_events::WsAccPosition;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestClearinghouseStateHyperliquid {
    pub assetPositions: Vec<RestAssetPositionHyperliquid>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestAssetPositionHyperliquid {
    #[serde(rename = "type")]
    pub kind: String,
    pub position: RestPositionHyperliquid,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestPositionHyperliquid {
    pub coin: String,
    #[serde(default)]
    pub entryPx: Value,
    pub leverage: RestLeverageHyperliquid,
    #[serde(default)]
    pub marginUsed: Value,
    #[serde(default)]
    pub szi: Value,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RestLeverageHyperliquid {
    #[serde(rename = "type")]
    pub kind: String,
    pub value: f64,
}

impl RestAssetPositionHyperliquid {
    pub fn into_position_data(self, mark_price: f64) -> PositionData {
        let size = value_to_f64(&self.position.szi);

        PositionData {
            timestamp: 0,
            inst: hyperliquid_perp_to_cli(&self.position.coin),
            inst_type: InstrumentType::Perpetual,
            position_side: if size > 0.0 {
                PositionSide::Long
            } else if size < 0.0 {
                PositionSide::Short
            } else {
                PositionSide::Both
            },
            size,
            avg_price: value_to_f64(&self.position.entryPx),
            mark_price,
            margin: value_to_f64(&self.position.marginUsed),
            leverage: self.position.leverage.value,
        }
    }

    pub fn into_ws_position(self) -> WsAccPosition {
        let size = value_to_f64(&self.position.szi);

        WsAccPosition {
            inst: hyperliquid_perp_to_cli(&self.position.coin),
            inst_type: InstrumentType::Perpetual,
            avg_price: value_to_f64(&self.position.entryPx),
            size,
            position_side: if size > 0.0 {
                PositionSide::Long
            } else if size < 0.0 {
                PositionSide::Short
            } else {
                PositionSide::Both
            },
            margin_mode: match self.position.leverage.kind.as_str() {
                "cross" => MarginMode::Cross,
                "isolated" => MarginMode::Isolated,
                _ => MarginMode::Unknown,
            },
        }
    }
}
