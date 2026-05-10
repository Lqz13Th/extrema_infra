use serde::Deserialize;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestAssetBalanceOkx {
    #[serde(default)]
    pub ccy: String,
    #[serde(default)]
    pub bal: String,
    #[serde(default)]
    pub frozenBal: String,
    #[serde(default)]
    pub availBal: String,
}
