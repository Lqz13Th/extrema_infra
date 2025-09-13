use serde::Deserialize;

use crate::strategy_base::event_notify::cex_notify::*;
use crate::market_assets::{
    cex::binance::api_utils::binance_um_to_perp_symbol,
    base_data::*
};
use crate::traits::conversion::IntoWsData;

#[allow(non_snake_case)]
#[derive(Debug, Clone, Deserialize)]
pub struct WsAggTradeBinanceUM {
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
    type Output = Vec<WsTrade>;
    fn into_ws(self) -> Vec<WsTrade> {
        let trade = WsTrade {
            timestamp: self.T,
            market: Market::BinanceUmFutures,
            symbol: binance_um_to_perp_symbol(&self.s),
            price: self.p.parse().unwrap_or(0.0),
            size: self.q.parse().unwrap_or(0.0),
            side: if self.m { Side::SELL } else { Side::BUY },
            trade_id: self.a,
        };

        vec![trade]
    }
}

