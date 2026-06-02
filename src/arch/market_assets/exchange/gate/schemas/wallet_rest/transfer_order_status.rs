use serde::Deserialize;

use crate::arch::market_assets::api_general::de_string_from_any;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestTransferOrderStatusGate {
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub tx_id: String,
    #[serde(default)]
    pub status: String,
}
