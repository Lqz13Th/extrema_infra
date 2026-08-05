use serde::Deserialize;

use crate::arch::{
    market_assets::{
        api_general::ts_to_micros, base_data::OrderSide, exchange::okx::api_utils::okx_inst_to_cli,
        market_core::Market,
    },
    strategy_base::handler::lob_events::WsTrade,
    traits::conversion::IntoWsData,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsTradesOkx {
    instId: String,
    tradeId: String,
    px: String,
    sz: String,
    side: String,
    ts: String,
}

impl IntoWsData for WsTradesOkx {
    type Output = WsTrade;

    fn into_ws(self) -> Self::Output {
        WsTrade {
            timestamp: ts_to_micros(self.ts.parse().unwrap_or_default()),
            market: Market::Okx,
            inst: okx_inst_to_cli(&self.instId),
            price: self.px.parse().unwrap_or_default(),
            size: self.sz.parse().unwrap_or_default(),
            side: match self.side.as_str() {
                "buy" => OrderSide::BUY,
                "sell" => OrderSide::SELL,
                _ => OrderSide::Unknown,
            },
            trade_id: self.tradeId.parse().unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::arch::market_assets::exchange::okx::okx_ws_msg::OkxWsData;

    use super::*;

    #[test]
    fn decodes_trade_batch_frame() {
        let frame = br#"{
            "arg":{"channel":"trades","instId":"BTC-USDT-SWAP"},
            "data":[{
                "instId":"BTC-USDT-SWAP","tradeId":"987654321",
                "px":"63405.40","sz":"0.125","side":"buy","ts":"1780563843113"
            }]
        }"#;

        let trade = OkxWsData::<WsTradesOkx>::decode_batch(frame)
            .unwrap()
            .into_ws()
            .pop()
            .unwrap();

        assert_eq!(trade.inst, "BTC_USDT_PERP");
        assert_eq!(trade.price, 63_405.40);
        assert_eq!(trade.size, 0.125);
        assert_eq!(trade.side, OrderSide::BUY);
        assert_eq!(trade.trade_id, 987654321);
    }
}
