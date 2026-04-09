use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestWithdrawAddressBinance {
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub addressTag: String,
    #[serde(default)]
    pub coin: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub origin: String,
    #[serde(default)]
    pub originType: String,
    #[serde(default)]
    pub whiteStatus: bool,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
