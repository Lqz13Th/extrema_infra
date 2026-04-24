use serde::Deserialize;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestPriceLimitOkx {
    pub instType: String,
    pub instId: String,
    pub buyLmt: String,
    pub sellLmt: String,
    pub ts: String,
    #[serde(default)]
    pub enabled: Option<bool>,
}
