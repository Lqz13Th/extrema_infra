use serde::Deserialize;
use serde_json::Value;

/// Market-level ADL risk state of one contract: `normal`, `warning` or `adl_risk`.
///
/// Gate pushes a full snapshot with `event: "all"` right after subscribing and
/// `event: "update"` frames afterwards.
#[derive(Clone, Debug, Deserialize)]
pub struct WsAdlWarningGateFutures {
    pub contract: String,
    pub settle: String,
    pub state: String,
    #[serde(default)]
    pub update_time: Option<Value>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::arch::market_assets::exchange::gate::api_utils::parse_ws_adl_warning_gate;

    #[test]
    fn parses_update_frame() {
        let frame = json!({
            "time": 1788433195,
            "time_ms": 1788433195394u64,
            "channel": "futures.adl_warning",
            "event": "update",
            "result": [
                { "contract": "AKE_USDT", "settle": "usdt", "state": "warning", "update_time": 1788433195 },
                { "contract": "BTW_USDT", "settle": "usdt", "state": "adl_risk", "update_time": 1788433195 }
            ]
        })
        .to_string();

        let data = parse_ws_adl_warning_gate(&frame).unwrap();

        assert_eq!(data.len(), 2);
        assert_eq!(data[0].contract, "AKE_USDT");
        assert_eq!(data[0].settle, "usdt");
        assert_eq!(data[0].state, "warning");
        assert_eq!(data[0].update_time, Some(json!(1788433195)));
        assert_eq!(data[1].state, "adl_risk");
    }

    #[test]
    fn parses_single_object_result() {
        let frame = json!({
            "time": 1788433195,
            "channel": "futures.adl_warning",
            "event": "update",
            "result": { "contract": "AKE_USDT", "settle": "usdt", "state": "normal" }
        })
        .to_string();

        let data = parse_ws_adl_warning_gate(&frame).unwrap();

        assert_eq!(data.len(), 1);
        assert_eq!(data[0].state, "normal");
        assert_eq!(data[0].update_time, None);
    }

    #[test]
    fn subscribe_ack_yields_no_data() {
        let frame = json!({
            "time": 1788433195,
            "channel": "futures.adl_warning",
            "event": "subscribe",
            "payload": ["!all"],
            "result": { "status": "success" }
        })
        .to_string();

        assert!(parse_ws_adl_warning_gate(&frame).unwrap().is_empty());
    }
}
