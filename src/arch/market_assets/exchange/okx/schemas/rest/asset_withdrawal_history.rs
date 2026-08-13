use std::collections::HashMap;

use serde::Deserialize;

use crate::arch::market_assets::api_general::de_micros_from_int;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestAssetWithdrawalHistoryOkx {
    #[serde(default)]
    pub ccy: String,
    #[serde(default)]
    pub chain: String,
    #[serde(default)]
    pub amt: String,
    #[serde(default)]
    pub fee: String,
    #[serde(default)]
    pub feeCcy: String,
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub areaCodeFrom: String,
    #[serde(default)]
    pub to: String,
    #[serde(default)]
    pub areaCodeTo: String,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub memo: String,
    #[serde(default)]
    pub pmtId: String,
    #[serde(default)]
    pub addrEx: HashMap<String, String>,
    #[serde(default)]
    pub txId: String,
    #[serde(default, deserialize_with = "de_micros_from_int")]
    pub ts: u64,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub wdId: String,
    #[serde(default)]
    pub clientId: String,
    #[serde(default)]
    pub nonTradableAsset: bool,
    #[serde(default)]
    pub r#type: String,
}
