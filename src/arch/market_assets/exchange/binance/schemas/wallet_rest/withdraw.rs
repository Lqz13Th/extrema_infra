use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestWithdrawBinance {
    pub id: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
