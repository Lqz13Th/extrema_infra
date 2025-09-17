use serde::Deserialize;

use crate::market_assets::{
    market_core::Market,
    base_data::*,
    api_general::ts_to_micros,
    cex::binance::api_utils::binance_um_to_cli_perp,
};
use crate::strategy_base::handler::cex_events::WsTrade;
use crate::traits::conversion::IntoWsData;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct WsAggTradeBinanceUM {
    pub e: String,      // Event type
    pub E: u64,         // Event time
    pub a: u64,         // Aggregate trade ID
    pub s: String,      // Symbol
    pub p: String,      // Price
    pub q: String,      // Quantity
    pub f: u64,         // First trade ID
    pub l: u64,         // Last trade ID
    pub T: u64,         // Trade time
    pub m: bool,        // Is the buyer the market maker?
    pub M: bool,        // Ignore
}

impl IntoWsData for WsAggTradeBinanceUM {
    type Output = WsTrade;
    fn into_ws(self) -> WsTrade {
        WsTrade {
            timestamp: ts_to_micros(self.T),
            market: Market::BinanceUmFutures,
            symbol: binance_um_to_cli_perp(&self.s),
            price: self.p.parse().unwrap_or(0.0),
            size: self.q.parse().unwrap_or(0.0),
            side: if self.m { OrderSide::SELL } else { OrderSide::BUY },
            trade_id: self.a,
        }
    }
}

