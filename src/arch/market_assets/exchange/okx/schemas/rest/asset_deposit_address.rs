use serde::Deserialize;
use std::collections::HashMap;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestAssetDepositAddressOkx {
    #[serde(default)]
    pub ccy: String,
    #[serde(default)]
    pub chain: String,
    #[serde(default)]
    pub ctAddr: String,
    #[serde(default)]
    pub addr: String,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub memo: String,
    #[serde(default)]
    pub pmtId: String,
    #[serde(default)]
    pub addrEx: HashMap<String, String>,
    #[serde(default)]
    pub to: String,
    #[serde(default)]
    pub selected: bool,
}
