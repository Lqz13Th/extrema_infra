use serde::Deserialize;

use crate::arch::{
    market_assets::{
        api_general::ts_to_micros, base_data::*,
        exchange::binance::api_utils::binance_fut_inst_to_cli, market_core::Market,
    },
    strategy_base::handler::lob_events::WsTrade,
    traits::conversion::IntoWsData,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsAggTradeBinanceUM {
    e: String, // Event type
    E: u64,    // Event time
    a: u64,    // Aggregate trade ID
    s: String, // Symbol
    p: String, // Price
    q: String, // Quantity
    f: u64,    // First trade ID
    l: u64,    // Last trade ID
    T: u64,    // Trade time
    m: bool,   // Is the buyer the market maker?
}

impl IntoWsData for WsAggTradeBinanceUM {
    type Output = WsTrade;
    fn into_ws(self) -> WsTrade {
        WsTrade {
            timestamp: ts_to_micros(self.T),
            market: Market::BinanceUmFutures,
            inst: binance_fut_inst_to_cli(&self.s),
            price: self.p.parse().unwrap_or_default(),
            size: self.q.parse().unwrap_or_default(),
            side: if self.m {
                OrderSide::SELL
            } else {
                OrderSide::BUY
            },
            trade_id: self.a,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::arch::market_assets::exchange::binance::binance_ws_msg::BinanceWsData;

    use super::*;

    #[test]
    fn decodes_aggregate_trade_frame() {
        let frame = br#"{
            "e":"aggTrade","E":1780563843114,"a":987654321,
            "s":"BTCUSDT","p":"63405.40","q":"0.125",
            "f":100,"l":101,"T":1780563843113,"m":true
        }"#;

        let trade = BinanceWsData::<WsAggTradeBinanceUM>::decode_single(frame)
            .unwrap()
            .into_ws()
            .pop()
            .unwrap();

        assert_eq!(trade.inst, "BTC_USDT_PERP");
        assert_eq!(trade.price, 63_405.40);
        assert_eq!(trade.size, 0.125);
        assert_eq!(trade.side, OrderSide::SELL);
        assert_eq!(trade.trade_id, 987654321);
    }
}
