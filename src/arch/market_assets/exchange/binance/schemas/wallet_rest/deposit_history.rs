use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::arch::market_assets::api_general::de_u64_from_string_or_number;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestDepositHistoryBinance {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub amount: String,
    #[serde(default)]
    pub coin: String,
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub status: i32,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub addressTag: String,
    #[serde(default)]
    pub txId: String,
    #[serde(default, deserialize_with = "de_u64_from_string_or_number")]
    pub insertTime: u64,
    #[serde(default, deserialize_with = "de_u64_from_string_or_number")]
    pub completeTime: u64,
    #[serde(default)]
    pub confirmTimes: String,
    #[serde(default)]
    pub walletType: i32,
    #[serde(default)]
    pub travelRuleStatus: i32,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
