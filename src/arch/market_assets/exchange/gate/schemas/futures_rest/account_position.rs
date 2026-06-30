use serde::Deserialize;
use serde_json::Value;

use crate::arch::market_assets::{
    api_data::account_data::PositionData,
    api_general::{ts_to_micros, value_to_f64},
    base_data::{InstrumentType, PositionSide},
    exchange::gate::api_utils::gate_fut_inst_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestAccountPosGateFutures {
    pub contract: String,
    pub size: Value,
    pub entry_price: Option<Value>,
    pub mark_price: Option<Value>,
    pub initial_margin: Option<Value>,
    pub lever: Option<Value>,
    pub update_time: Option<Value>,
}

impl From<RestAccountPosGateFutures> for PositionData {
    fn from(d: RestAccountPosGateFutures) -> Self {
        let size = value_to_f64(&d.size);

        PositionData {
            timestamp: ts_to_micros(
                d.update_time.as_ref().map(value_to_f64).unwrap_or_default() as u64
            ),
            inst: gate_fut_inst_to_cli(&d.contract),
            inst_type: InstrumentType::Perpetual,
            position_side: if size > 0.0 {
                PositionSide::Long
            } else if size < 0.0 {
                PositionSide::Short
            } else {
                PositionSide::Both
            },
            size,
            avg_price: d.entry_price.as_ref().map(value_to_f64).unwrap_or_default(),
            mark_price: d.mark_price.as_ref().map(value_to_f64).unwrap_or_default(),
            margin: d
                .initial_margin
                .as_ref()
                .map(value_to_f64)
                .unwrap_or_default(),
            leverage: d.lever.as_ref().map(value_to_f64).unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::arch::market_assets::base_data::PositionSide;

    use super::*;

    #[test]
    fn converts_decimal_contract_position_size() {
        let raw: RestAccountPosGateFutures = serde_json::from_value(json!({
            "contract": "LAB_USDT",
            "size": "0.1",
            "entry_price": "14.64758",
            "mark_price": "13.8101",
            "initial_margin": "27.72377575",
            "lever": "5",
            "update_time": 1782726621
        }))
        .unwrap();

        let position = PositionData::from(raw);

        assert_eq!(position.inst, "LAB_USDT_PERP");
        assert_eq!(position.position_side, PositionSide::Long);
        assert_eq!(position.size, 0.1);
        assert_eq!(position.avg_price, 14.64758);
        assert_eq!(position.mark_price, 13.8101);
    }
}
