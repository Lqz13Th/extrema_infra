use serde::Deserialize;
use std::collections::HashMap;

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
    #[serde(default)]
    pub ts: String,
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
