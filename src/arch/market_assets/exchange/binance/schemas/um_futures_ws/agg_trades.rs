use serde::Deserialize;

use crate::arch::{
    market_assets::{
        api_general::ts_to_micros, base_data::*, exchange::binance::api_utils::binance_inst_to_cli,
        market_core::Market,
    },
    strategy_base::handler::cex_events::WsTrade,
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
            inst: binance_inst_to_cli(&self.s),
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
