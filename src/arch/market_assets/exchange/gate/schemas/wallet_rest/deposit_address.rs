use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::arch::market_assets::api_general::de_string_from_any;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestMultiChainAddressGate {
    #[serde(default)]
    pub chain: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub payment_id: String,
    #[serde(default)]
    pub payment_name: String,
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub obtain_failed: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestDepositAddressGate {
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub multichain_addresses: Vec<RestMultiChainAddressGate>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
