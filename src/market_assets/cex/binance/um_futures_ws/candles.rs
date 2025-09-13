use serde::Deserialize;

use crate::market_assets::base_data::Market;
use crate::market_assets::cex::binance::api_utils::binance_um_to_perp_symbol;
use crate::strategy_base::event_notify::cex_notify::WsCandle;
use crate::traits::conversion::IntoWsData;

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct WsCandleBinanceUM {
    pub ps: String,       // Pair
    pub k: KlineDetails,
}


#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct KlineDetails {
    pub t: u64,           // Kline start time
    pub i: String,        // Interval
    pub o: String,        // Open price
    pub c: String,        // Close price
    pub h: String,        // High price
    pub l: String,        // Low price
    pub v: String,        // Volume
    pub x: bool,          // Is this kline closed?
}

impl IntoWsData for WsCandleBinanceUM {
    type Output = Vec<WsCandle>;
    fn into_ws(self) -> Vec<WsCandle> {
        let candle = WsCandle {
            timestamp: self.k.t,
            market: Market::BinanceUmFutures,
            symbol: binance_um_to_perp_symbol(&self.ps),
            interval: self.k.i,
            open: self.k.o.parse().unwrap_or(0.0),
            high: self.k.h.parse().unwrap_or(0.0),
            low: self.k.l.parse().unwrap_or(0.0),
            close: self.k.c.parse().unwrap_or(0.0),
            volume: self.k.v.parse().unwrap_or(0.0),
            confirm: self.k.x,
        };

        vec![candle]
    }
}