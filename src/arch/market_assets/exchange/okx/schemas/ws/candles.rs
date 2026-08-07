use serde::Deserialize;

use crate::arch::{
    market_assets::{
        api_general::ts_to_micros,
        exchange::okx::{
            api_utils::okx_inst_to_cli,
            okx_ws_msg::{IntoOkxWsData, WsArg},
        },
        market_core::Market,
    },
    strategy_base::handler::lob_events::WsCandle,
    task_execution::task_ws::CandleParam,
};

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsCandleOkx([String; 9]);

impl IntoOkxWsData for WsCandleOkx {
    type Output = WsCandle;

    fn into_ws_with_okx_context(self, arg: &WsArg, _action: Option<&str>) -> Self::Output {
        let [
            timestamp,
            open,
            high,
            low,
            close,
            volume,
            _volume_ccy,
            _volume_quote,
            confirm,
        ] = self.0;
        let interval = arg
            .channel
            .as_deref()
            .and_then(|channel| channel.strip_prefix("candle"))
            .map(candle_param_from_okx_interval)
            .unwrap_or(CandleParam::OneMinute);

        WsCandle {
            timestamp: ts_to_micros(timestamp.parse().unwrap_or_default()),
            market: Market::Okx,
            inst: okx_inst_to_cli(arg.instId.as_deref().unwrap_or_default()),
            interval,
            open: open.parse().unwrap_or_default(),
            high: high.parse().unwrap_or_default(),
            low: low.parse().unwrap_or_default(),
            close: close.parse().unwrap_or_default(),
            volume: volume.parse().unwrap_or_default(),
            confirm: confirm == "1",
        }
    }
}

fn candle_param_from_okx_interval(interval: &str) -> CandleParam {
    match interval {
        "1s" => CandleParam::OneSecond,
        "1m" => CandleParam::OneMinute,
        "5m" => CandleParam::FiveMinutes,
        "15m" => CandleParam::FifteenMinutes,
        "1H" => CandleParam::OneHour,
        "4H" => CandleParam::FourHours,
        "1D" | "1Dutc" => CandleParam::OneDay,
        "1W" | "1Wutc" => CandleParam::OneWeek,
        custom => CandleParam::Custom(custom.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use crate::arch::{
        market_assets::exchange::okx::okx_ws_msg::OkxWsData, traits::conversion::IntoWsData,
    };

    use super::*;

    #[test]
    fn decodes_okx_candle_batch_with_outer_context() {
        let frame = br#"{
            "arg":{"channel":"candle1Dutc","instId":"BTC-USDT-SWAP"},
            "data":[[
                "1597026383085","8533.02","8553.74","8527.17","8548.26",
                "45247","529.5858061","5529.5858061","1"
            ]]
        }"#;

        let candles = OkxWsData::<WsCandleOkx>::decode_batch(frame)
            .unwrap()
            .into_ws();

        assert_eq!(candles.len(), 1);
        let candle = &candles[0];
        assert_eq!(candle.timestamp, 1_597_026_383_085_000);
        assert_eq!(candle.market, Market::Okx);
        assert_eq!(candle.inst, "BTC_USDT_PERP");
        assert_eq!(candle.interval, CandleParam::OneDay);
        assert_eq!(candle.open, 8533.02);
        assert_eq!(candle.high, 8553.74);
        assert_eq!(candle.low, 8527.17);
        assert_eq!(candle.close, 8548.26);
        assert_eq!(candle.volume, 45247.0);
        assert!(candle.confirm);
    }
}
