use serde::Deserialize;
use serde_json::Value;

use crate::arch::{
    market_assets::{
        api_general::{ts_to_micros, value_to_f64},
        base_data::OrderSide,
        exchange::gate::api_utils::gate_fut_inst_to_cli,
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
            inst: gate_fut_inst_to_cli(&self.contract),
            price: value_to_f64(&self.price),
            size: size_val.abs(),
            side,
            trade_id: self.id,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::arch::market_assets::exchange::gate::gate_ws_msg::GateWsData;

    use super::*;

    #[test]
    fn decodes_trade_batch_frame() {
        let frame = br#"{
            "channel":"futures.trades","event":"update","result":[{
                "contract":"BTC_USDT","size":-3,"id":987654321,
                "create_time_ms":1780563843113,"price":"63405.40"
            }]
        }"#;

        let trade = GateWsData::<WsTradeGateFutures>::decode_batch(frame)
            .unwrap()
            .into_ws()
            .pop()
            .unwrap();

        assert_eq!(trade.inst, "BTC_USDT_PERP");
        assert_eq!(trade.price, 63_405.40);
        assert_eq!(trade.size, 3.0);
        assert_eq!(trade.side, OrderSide::SELL);
        assert_eq!(trade.trade_id, 987654321);
    }
}
