use serde::Deserialize;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestAssetTransferOkx {
    #[serde(default)]
    pub transId: String,
    #[serde(default)]
    pub clientId: String,
    #[serde(default)]
    pub ccy: String,
    #[serde(default)]
    pub amt: String,
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub to: String,
    #[serde(default)]
    pub subAcct: String,
    #[serde(default)]
    pub r#type: String,
}
