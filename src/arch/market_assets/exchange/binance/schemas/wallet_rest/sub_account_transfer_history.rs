use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::arch::market_assets::api_general::{de_micros_from_int, de_u64_from_string_or_number};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestSubAccountTransferHistoryRowBinance {
    #[serde(default, deserialize_with = "de_u64_from_string_or_number")]
    pub tranId: u64,
    #[serde(default)]
    pub fromEmail: String,
    #[serde(default)]
    pub toEmail: String,
    #[serde(default)]
    pub asset: String,
    #[serde(default)]
    pub amount: String,
    #[serde(default, deserialize_with = "de_micros_from_int")]
    pub createTimeStamp: u64,
    #[serde(default)]
    pub fromAccountType: String,
    #[serde(default)]
    pub toAccountType: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub clientTranId: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestSubAccountTransferHistoryBinance {
    #[serde(default)]
    pub result: Vec<RestSubAccountTransferHistoryRowBinance>,
    #[serde(default, deserialize_with = "de_u64_from_string_or_number")]
    pub totalCount: u64,
}
