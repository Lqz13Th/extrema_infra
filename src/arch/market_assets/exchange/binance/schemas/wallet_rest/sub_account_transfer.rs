use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::arch::market_assets::api_general::de_u64_from_string_or_number;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestSubAccountUniversalTransferBinance {
    #[serde(default, deserialize_with = "de_u64_from_string_or_number")]
    pub tranId: u64,
    #[serde(default)]
    pub clientTranId: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
