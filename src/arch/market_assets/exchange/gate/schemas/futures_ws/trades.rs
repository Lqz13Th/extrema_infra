use serde::Deserialize;
use serde_json::Value;

use crate::arch::{
    market_assets::{
        api_general::{ts_to_micros, value_to_f64},
        base_data::OrderSide,
        exchange::gate::api_utils::gate_inst_to_cli,
        market_core::Market,
    },
    strategy_base::handler::lob_events::WsTrade,
    traits::conversion::IntoWsData,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsTradeGateFutures {
    contract: String,
    size: Value,
    id: u64,
    create_time: Option<u64>,
    create_time_ms: Option<u64>,
    price: Value,
}

impl IntoWsData for WsTradeGateFutures {
    type Output = WsTrade;

    fn into_ws(self) -> WsTrade {
        let size_val = value_to_f64(&self.size);
        let side = if size_val >= 0.0 {
            OrderSide::BUY
        } else {
            OrderSide::SELL
        };
        let timestamp = self
            .create_time_ms
            .map(ts_to_micros)
            .or_else(|| self.create_time.map(ts_to_micros))
            .unwrap_or_default();

        WsTrade {
            timestamp,
            market: Market::GateFutures,
            inst: gate_inst_to_cli(&self.contract),
            price: value_to_f64(&self.price),
            size: size_val.abs(),
            side,
            trade_id: self.id,
        }
    }
}
