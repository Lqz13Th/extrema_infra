use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use crate::arch::market_assets::{
    api_data::account_data::PositionData,
    api_general::{get_micros_timestamp, value_to_f64},
    base_data::{InstrumentType, PositionSide},
    exchange::hyperliquid::api_utils::hyperliquid_perp_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestClearinghouseStateHyperliquid {
    pub assetPositions: Vec<RestAssetPositionHyperliquid>,
}

impl RestClearinghouseStateHyperliquid {
    pub fn into_position_data(self, mark_px_by_coin: &HashMap<String, f64>) -> Vec<PositionData> {
        self.assetPositions
            .into_iter()
            .map(|position| {
                let mark_price = mark_px_by_coin
                    .get(&position.position.coin)
                    .copied()
                    .unwrap_or_default();
                position.into_position_data(mark_price)
            })
            .collect()
    }
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
            timestamp: get_micros_timestamp(),
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
}
