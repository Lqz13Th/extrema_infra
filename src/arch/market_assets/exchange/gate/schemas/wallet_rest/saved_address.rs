use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestSavedAddressGate {
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub chain: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub memo: String,
    #[serde(default)]
    pub verified: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
