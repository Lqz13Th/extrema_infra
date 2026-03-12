use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestUserUniversalTransferBinance {
    pub tranId: u64,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
