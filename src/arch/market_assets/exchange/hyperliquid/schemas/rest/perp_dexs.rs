use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct RestPerpDexHyperliquid {
    pub name: String,
}
