use serde::Deserialize;

use crate::arch::{
    market_assets::{
        api_general::ts_to_micros, base_data::OrderSide,
        exchange::hyperliquid::api_utils::hyperliquid_inst_to_cli, market_core::Market,
    },
    strategy_base::handler::lob_events::WsTrade,
    traits::conversion::IntoWsData,
};

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsTradeHyperliquid {
    coin: String,
    side: String,
    px: String,
    sz: String,
    time: u64,
    tid: u64,
}

impl IntoWsData for WsTradeHyperliquid {
    type Output = WsTrade;

    fn into_ws(self) -> Self::Output {
        WsTrade {
            timestamp: ts_to_micros(self.time),
            market: Market::HyperLiquid,
            inst: hyperliquid_inst_to_cli(&self.coin),
            price: self.px.parse().unwrap_or_default(),
            size: self.sz.parse().unwrap_or_default(),
            side: match self.side.as_str() {
                "B" => OrderSide::BUY,
                "A" => OrderSide::SELL,
                _ => OrderSide::Unknown,
            },
            trade_id: self.tid,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::arch::market_assets::exchange::hyperliquid::hyperliquid_ws_msg::HyperliquidWsData;

    use super::*;

    #[test]
    fn decodes_trade_batch_frame() {
        let frame = br#"{
            "channel":"trades","data":[{
                "coin":"BTC","side":"B","px":"63405.40","sz":"0.125",
                "time":1780563843113,"tid":987654321
            }]
        }"#;

        let trade = HyperliquidWsData::<WsTradeHyperliquid>::decode_batch(frame)
            .unwrap()
            .into_ws()
            .pop()
            .unwrap();

        assert_eq!(trade.inst, "BTC_USDC_PERP");
        assert_eq!(trade.price, 63_405.40);
        assert_eq!(trade.size, 0.125);
        assert_eq!(trade.side, OrderSide::BUY);
        assert_eq!(trade.trade_id, 987654321);
    }
}
