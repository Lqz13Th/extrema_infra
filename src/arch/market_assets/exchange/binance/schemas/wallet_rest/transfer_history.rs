use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::arch::market_assets::api_general::{de_micros_from_int, de_u64_from_string_or_number};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestTransferHistoryRowBinance {
    #[serde(default)]
    pub asset: String,
    #[serde(default)]
    pub amount: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub status: String,
    #[serde(default, deserialize_with = "de_u64_from_string_or_number")]
    pub tranId: u64,
    #[serde(default, deserialize_with = "de_micros_from_int")]
    pub timestamp: u64,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestTransferHistoryBinance {
    #[serde(default)]
    pub total: i64,
    #[serde(default)]
    pub rows: Vec<RestTransferHistoryRowBinance>,
}
