use serde::Deserialize;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestPositionModeBinanceUM {
    pub dualSidePosition: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RestPositionModeChangeBinanceUM {
    pub code: i64,
    pub msg: String,
}
