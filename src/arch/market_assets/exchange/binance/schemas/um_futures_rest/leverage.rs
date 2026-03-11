use serde::Deserialize;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestLeverageBinanceUM {
    pub leverage: u32,
    pub maxNotionalValue: String,
    pub symbol: String,
}
