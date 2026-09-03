use serde::Deserialize;
use serde_json::Value;

/// Market-level ADL state of one instrument family: `normal`, `warning` or `adl`.
///
/// Since 2026-06 OKX no longer pushes this channel while a family is in the
/// `normal` state, so the absence of frames means normal and consumers must age
/// out a previously received `warning` or `adl` state themselves. `maxBal`,
/// `bal`, `decRate` and `adlType` currently arrive as empty strings.
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct WsAdlWarningOkx {
    pub instType: String,
    pub instFamily: String,
    pub state: String,
    #[serde(default)]
    pub maxBal: Option<Value>,
    #[serde(default)]
    pub bal: Option<Value>,
    #[serde(default)]
    pub decRate: Option<Value>,
    #[serde(default)]
    pub adlType: Option<Value>,
    #[serde(default)]
    pub ts: Option<String>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::arch::market_assets::exchange::okx::api_utils::parse_ws_adl_warning_okx;

    #[test]
    fn parses_warning_frame_with_empty_fields() {
        let frame = json!({
            "arg": { "channel": "adl-warning", "instType": "SWAP", "instFamily": "BTC-USDT" },
            "data": [{
                "instType": "SWAP",
                "instFamily": "BTC-USDT",
                "state": "warning",
                "maxBal": "",
                "bal": "",
                "decRate": "",
                "adlType": "",
                "ts": "1788433195394"
            }]
        })
        .to_string();

        let data = parse_ws_adl_warning_okx(&frame).unwrap();

        assert_eq!(data.len(), 1);
        assert_eq!(data[0].instType, "SWAP");
        assert_eq!(data[0].instFamily, "BTC-USDT");
        assert_eq!(data[0].state, "warning");
        assert_eq!(data[0].ts.as_deref(), Some("1788433195394"));
        assert_eq!(data[0].maxBal, Some(json!("")));
    }

    #[test]
    fn subscribe_ack_yields_no_data() {
        let frame = json!({
            "event": "subscribe",
            "arg": { "channel": "adl-warning", "instType": "SWAP" },
            "connId": "a4d3ae55"
        })
        .to_string();

        assert!(parse_ws_adl_warning_okx(&frame).unwrap().is_empty());
    }
}
