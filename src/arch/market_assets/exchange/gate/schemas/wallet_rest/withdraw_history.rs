use serde::Deserialize;

use crate::arch::market_assets::api_general::{de_micros_from_int, de_string_from_any};

#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestWithdrawHistoryGate {
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub id: String,
    #[serde(default)]
    pub txid: String,
    #[serde(default)]
    pub block_number: String,
    #[serde(default)]
    pub withdraw_order_id: String,
    #[serde(default, deserialize_with = "de_micros_from_int")]
    pub timestamp: u64,
    #[serde(default)]
    pub amount: String,
    #[serde(default)]
    pub fee: String,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub fail_reason: String,
    #[serde(default, deserialize_with = "de_micros_from_int")]
    pub timestamp2: u64,
    #[serde(default)]
    pub memo: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub chain: String,
}
