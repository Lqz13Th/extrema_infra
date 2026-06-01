use serde::Deserialize;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestSymbolConfigBinanceUM {
    pub symbol: String,
    pub marginType: String,
    pub isAutoAddMargin: bool,
    pub leverage: u32,
    pub maxNotionalValue: String,
}
