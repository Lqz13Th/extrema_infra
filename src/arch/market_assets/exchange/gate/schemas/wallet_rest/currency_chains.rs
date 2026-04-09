use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::arch::market_assets::api_general::de_string_from_any;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestCurrencyChainGate {
    #[serde(default)]
    pub chain: String,
    #[serde(default)]
    pub name_cn: String,
    #[serde(default)]
    pub name_en: String,
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub is_disabled: String,
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub is_deposit_disabled: String,
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub is_withdraw_disabled: String,
    #[serde(default)]
    pub reason: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
