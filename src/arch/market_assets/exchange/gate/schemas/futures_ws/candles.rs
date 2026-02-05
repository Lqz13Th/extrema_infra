use serde::Deserialize;
use serde_json::Value;

use crate::arch::{
    market_assets::{
        api_general::{ts_to_micros, value_to_f64},
        exchange::gate::api_utils::gate_inst_to_cli,
        market_core::Market,
    },
    strategy_base::handler::lob_events::WsCandle,
    task_execution::task_ws::CandleParam,
    traits::conversion::IntoWsData,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsCandleGateFutures {
    t: u64,
    v: Value,
    c: Value,
    h: Value,
    l: Value,
    o: Value,
    n: String,
    w: Option<bool>,
}

impl IntoWsData for WsCandleGateFutures {
    type Output = WsCandle;

    fn into_ws(self) -> WsCandle {
        let (interval_str, contract) = parse_candle_name(&self.n);
        let interval = CandleParam::from_candle_str(&interval_str)
            .unwrap_or(CandleParam::Custom(interval_str));

        WsCandle {
            timestamp: ts_to_micros(self.t),
            market: Market::GateFutures,
            inst: gate_inst_to_cli(&contract),
            interval,
            open: value_to_f64(&self.o),
            high: value_to_f64(&self.h),
            low: value_to_f64(&self.l),
            close: value_to_f64(&self.c),
            volume: value_to_f64(&self.v),
            confirm: self.w.unwrap_or(false),
        }
    }
}

fn parse_candle_name(name: &str) -> (String, String) {
    let mut parts = name.splitn(2, '_');
    let interval = parts.next().unwrap_or("1m").to_string();
    let contract = parts.next().unwrap_or("").to_string();
    (interval, contract)
}
