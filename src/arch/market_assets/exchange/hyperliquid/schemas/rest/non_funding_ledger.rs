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
    pub user: String,
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub destination: String,
    #[serde(rename = "sourceDex", default, deserialize_with = "de_string_from_any")]
    pub source_dex: String,
    #[serde(
        rename = "destinationDex",
        default,
        deserialize_with = "de_string_from_any"
    )]
    pub destination_dex: String,
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub token: String,
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub amount: String,
    #[serde(rename = "usdcValue", default, deserialize_with = "de_string_from_any")]
    pub usdc_value: String,
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

    #[test]
    fn parses_cctp_send_ledger_update() {
        let update: RestNonFundingLedgerUpdateHyperliquid =
            serde_json::from_value(serde_json::json!({
                "time": 1_786_205_432_100_u64,
                "hash": "0xdef456",
                "delta": {
                    "type": "send",
                    "user": "0x6B9E773128f453f5c2C60935Ee2DE2CBc5390A24",
                    "destination": "0x503bD29ef96555919B26E3810e59B25046c53176",
                    "sourceDex": "spot",
                    "destinationDex": "",
                    "token": "USDC",
                    "amount": "0.8",
                    "usdcValue": "0.8"
                }
            }))
            .unwrap();

        assert_eq!(update.delta.kind, "send");
        assert_eq!(
            update.delta.user,
            "0x6B9E773128f453f5c2C60935Ee2DE2CBc5390A24"
        );
        assert_eq!(
            update.delta.destination,
            "0x503bD29ef96555919B26E3810e59B25046c53176"
        );
        assert_eq!(update.delta.source_dex, "spot");
        assert_eq!(update.delta.destination_dex, "");
        assert_eq!(update.delta.token, "USDC");
        assert_eq!(update.delta.amount, "0.8");
        assert_eq!(update.delta.usdc_value, "0.8");
        assert_eq!(update.delta.usdc, "");
    }
}
