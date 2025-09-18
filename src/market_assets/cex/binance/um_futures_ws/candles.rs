use serde::Deserialize;

use crate::market_assets::{
    market_core::Market,
    api_general::ts_to_micros,
    cex::binance::api_utils::binance_um_to_cli_perp,
};
use crate::strategy_base::handler::cex_events::WsCandle;
use crate::task_execution::task_ws::CandleParam;
use crate::traits::conversion::IntoWsData;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsCandleBinanceUM {
    pub s: String,       // Pair
    pub k: KlineDetails,
}


#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct KlineDetails {
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
    type Output = WsCandle;
    fn into_ws(self) -> WsCandle {
        WsCandle {
            timestamp: ts_to_micros(self.k.t),
            market: Market::BinanceUmFutures,
            inst: binance_um_to_cli_perp(&self.s),
            interval: CandleParam::from_candle_str(&self.k.i)
                .unwrap_or(CandleParam::OneMinute),
            open: self.k.o.parse().unwrap_or(0.0),
            high: self.k.h.parse().unwrap_or(0.0),
            low: self.k.l.parse().unwrap_or(0.0),
            close: self.k.c.parse().unwrap_or(0.0),
            volume: self.k.v.parse().unwrap_or(0.0),
            confirm: self.k.x,
        }
    }
}