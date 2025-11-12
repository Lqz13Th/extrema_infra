use serde::Deserialize;

use crate::arch::{
    market_assets::{
        api_general::ts_to_micros,
        exchange::binance::api_utils::binance_inst_to_cli,
        market_core::Market,
    },
    strategy_base::handler::cex_events::WsCandle,
    task_execution::task_ws::CandleParam,
    traits::conversion::IntoWsData,
};

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
            inst: binance_inst_to_cli(&self.s),
            interval: CandleParam::from_candle_str(&self.k.i)
                .unwrap_or(CandleParam::OneMinute),
            open: self.k.o.parse().unwrap_or_default(),
            high: self.k.h.parse().unwrap_or_default(),
            low: self.k.l.parse().unwrap_or_default(),
            close: self.k.c.parse().unwrap_or_default(),
            volume: self.k.v.parse().unwrap_or_default(),
            confirm: self.k.x,
        }
    }
}