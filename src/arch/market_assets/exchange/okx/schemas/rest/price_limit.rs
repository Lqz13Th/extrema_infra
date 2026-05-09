use serde::Deserialize;

use crate::arch::market_assets::api_general::de_micros_from_int;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestPriceLimitOkx {
    pub instType: String,
    pub instId: String,
    pub buyLmt: String,
    pub sellLmt: String,
    #[serde(deserialize_with = "de_micros_from_int")]
    pub ts: u64,
    #[serde(default)]
    pub enabled: Option<bool>,
}
