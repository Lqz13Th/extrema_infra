use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::arch::market_assets::api_general::{de_string_from_any, de_u64_from_string_or_number};

#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestWithdrawGate {
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub id: String,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub amount: String,
    #[serde(default)]
    pub chain: String,
    #[serde(default)]
    pub memo: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub txid: String,
    #[serde(default)]
    pub fee: String,
    #[serde(default, deserialize_with = "de_u64_from_string_or_number")]
    pub timestamp: u64,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
