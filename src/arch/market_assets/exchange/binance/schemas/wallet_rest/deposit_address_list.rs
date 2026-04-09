use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::arch::market_assets::api_general::de_u64_from_string_or_number;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestDepositAddressListBinance {
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub coin: String,
    #[serde(default, alias = "addressTag")]
    pub tag: String,
    #[serde(default)]
    pub network: String,
    #[serde(default, deserialize_with = "de_u64_from_string_or_number")]
    pub isDefault: u64,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub url: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
