use serde::Deserialize;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestAssetWithdrawalOkx {
    #[serde(default)]
    pub wdId: String,
    #[serde(default)]
    pub clientId: String,
    #[serde(default)]
    pub ccy: String,
    #[serde(default)]
    pub amt: String,
    #[serde(default)]
    pub chain: String,
}
