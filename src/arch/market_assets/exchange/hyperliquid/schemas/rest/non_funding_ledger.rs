use serde::Deserialize;

use crate::arch::market_assets::api_general::{de_micros_from_int, de_string_from_any};

#[derive(Clone, Debug, Deserialize)]
pub struct RestNonFundingLedgerUpdateHyperliquid {
    #[serde(rename = "time", deserialize_with = "de_micros_from_int")]
    pub timestamp: u64,
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub hash: String,
    pub delta: RestNonFundingLedgerDeltaHyperliquid,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestNonFundingLedgerDeltaHyperliquid {
    #[serde(rename = "type", default, deserialize_with = "de_string_from_any")]
    pub kind: String,
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub usdc: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_deposit_ledger_update() {
        let update: RestNonFundingLedgerUpdateHyperliquid =
            serde_json::from_value(serde_json::json!({
                "time": 1_786_205_432_100_u64,
                "hash": "0xabc123",
                "delta": {
                    "type": "deposit",
                    "usdc": "79.89"
                }
            }))
            .unwrap();

        assert_eq!(update.timestamp, 1_786_205_432_100_000);
        assert_eq!(update.hash, "0xabc123");
        assert_eq!(update.delta.kind, "deposit");
        assert_eq!(update.delta.usdc, "79.89");
    }
}
