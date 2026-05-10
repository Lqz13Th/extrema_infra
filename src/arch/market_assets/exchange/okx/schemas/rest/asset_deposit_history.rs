use serde::Deserialize;

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
    #[serde(default)]
    pub ts: String,
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
