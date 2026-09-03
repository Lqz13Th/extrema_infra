use std::collections::HashMap;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct RestAdlRiskStatesGateFutures {
    pub settle: String,
    pub states: HashMap<String, RestAdlRiskStateGateFutures>,
}

/// Market-level ADL risk state of one contract: `normal`, `warning` or `adl_risk`.
#[derive(Clone, Debug, Deserialize)]
pub struct RestAdlRiskStateGateFutures {
    pub state: String,
    pub calculated_at_ms: u64,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parses_market_adl_risk_states() {
        let raw: RestAdlRiskStatesGateFutures = serde_json::from_value(json!({
            "settle": "usdt",
            "states": {
                "BTC_USDT": { "state": "normal", "calculated_at_ms": 1788433195394u64 },
                "AKE_USDT": { "state": "warning", "calculated_at_ms": 1788433195394u64 },
                "BTW_USDT": { "state": "adl_risk", "calculated_at_ms": 1788433195394u64 }
            }
        }))
        .unwrap();

        assert_eq!(raw.settle, "usdt");
        assert_eq!(raw.states.len(), 3);
        assert_eq!(raw.states["AKE_USDT"].state, "warning");
        assert_eq!(raw.states["BTW_USDT"].state, "adl_risk");
        assert_eq!(raw.states["BTC_USDT"].calculated_at_ms, 1788433195394);
    }
}
