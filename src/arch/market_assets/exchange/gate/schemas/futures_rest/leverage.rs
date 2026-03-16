use serde::Deserialize;
use serde_json::Value;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestLeverageGateFutures {
    pub contract: String,
    #[serde(default, alias = "lever")]
    pub leverage: Option<Value>,
    #[serde(default)]
    pub cross_leverage_limit: Option<Value>,
}
