use serde::Deserialize;

use crate::arch::market_assets::api_general::de_micros_from_int;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestAssetDepositHistoryOkx {
    #[serde(default)]
    pub ccy: String,
    #[serde(default)]
    pub chain: String,
    #[serde(default)]
    pub amt: String,
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub areaCodeFrom: String,
    #[serde(default)]
    pub to: String,
    #[serde(default)]
    pub areaCodeTo: String,
    #[serde(default)]
    pub txId: String,
    #[serde(default, deserialize_with = "de_micros_from_int")]
    pub ts: u64,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub depId: String,
    #[serde(default)]
    pub fromWdId: String,
    #[serde(default)]
    pub actualDepBlkConfirm: String,
    #[serde(default)]
    pub r#type: String,
}
