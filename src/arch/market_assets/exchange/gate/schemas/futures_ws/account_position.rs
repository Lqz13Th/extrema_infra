use serde::Deserialize;
use serde_json::Value;

use crate::arch::{
    market_assets::{
        api_general::value_to_f64,
        base_data::{InstrumentType, MarginMode, PositionSide},
        exchange::gate::api_utils::gate_fut_inst_to_cli,
    },
    strategy_base::handler::lob_events::WsAccPosition,
    traits::conversion::IntoWsData,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsAccountPositionGateFutures {
    contract: String,
    size: Value,
    entry_price: Option<Value>,
    pos_margin_mode: Option<String>,
}

impl IntoWsData for WsAccountPositionGateFutures {
    type Output = WsAccPosition;

    fn into_ws(self) -> WsAccPosition {
        let size_val = value_to_f64(&self.size);
        let avg_price = self
            .entry_price
            .as_ref()
            .map(value_to_f64)
            .unwrap_or_default();
        let mode = self.pos_margin_mode.unwrap_or_default();

        WsAccPosition {
            inst: gate_fut_inst_to_cli(&self.contract),
            inst_type: InstrumentType::Perpetual,
            avg_price,
            size: size_val,
            position_side: if size_val > 0.0 {
                PositionSide::Long
            } else if size_val < 0.0 {
                PositionSide::Short
            } else {
                PositionSide::Both
            },
            margin_mode: match mode.to_lowercase().as_str() {
                "cross" | "cross_margin" => MarginMode::Cross,
                "isolated" | "isolated_margin" => MarginMode::Isolated,
                _ => MarginMode::Unknown,
            },
        }
    }
}
