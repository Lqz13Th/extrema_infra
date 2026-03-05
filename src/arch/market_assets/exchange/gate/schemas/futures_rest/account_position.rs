use serde::Deserialize;
use serde_json::Value;

use crate::arch::market_assets::{
    api_data::account_data::PositionData,
    api_general::{ts_to_micros, value_to_f64},
    base_data::{InstrumentType, PositionSide},
    exchange::gate::api_utils::gate_inst_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestAccountPosGateFutures {
    pub contract: String,
    pub size: Value,
    pub entry_price: Option<Value>,
    pub mark_price: Option<Value>,
    pub margin: Option<Value>,
    pub leverage: Option<Value>,
    pub update_time: Option<Value>,
}

impl From<RestAccountPosGateFutures> for PositionData {
    fn from(d: RestAccountPosGateFutures) -> Self {
        let size = value_to_f64(&d.size);
        let avg_price = d.entry_price.as_ref().map(value_to_f64).unwrap_or_default();
        let mark_price = d.mark_price.as_ref().map(value_to_f64).unwrap_or_default();
        let margin = d.margin.as_ref().map(value_to_f64).unwrap_or_default();
        let leverage = d.leverage.as_ref().map(value_to_f64).unwrap_or_default();
        let ts = d.update_time.as_ref().map(value_to_f64).unwrap_or_default() as u64;

        PositionData {
            timestamp: ts_to_micros(ts),
            inst: gate_inst_to_cli(&d.contract),
            inst_type: InstrumentType::Perpetual,
            position_side: if size > 0.0 {
                PositionSide::Long
            } else if size < 0.0 {
                PositionSide::Short
            } else {
                PositionSide::Both
            },
            size,
            avg_price,
            mark_price,
            margin,
            leverage,
        }
    }
}
