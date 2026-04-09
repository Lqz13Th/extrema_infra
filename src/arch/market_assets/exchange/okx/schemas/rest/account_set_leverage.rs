use serde::Deserialize;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestAccountSetLeverageOkx {
    pub lever: String,
    pub mgnMode: String,
    #[serde(default)]
    pub instId: Option<String>,
    #[serde(default)]
    pub ccy: Option<String>,
    #[serde(default)]
    pub posSide: Option<String>,
}
