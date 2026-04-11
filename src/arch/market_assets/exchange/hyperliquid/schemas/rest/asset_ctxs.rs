use serde::Deserialize;
use serde_json::Value;

use super::{meta::RestMetaHyperliquid, spot_meta::RestSpotMetaHyperliquid};

#[derive(Clone, Debug, Deserialize)]
pub struct RestMetaAndAssetCtxsHyperliquid(
    pub RestMetaHyperliquid,
    pub Vec<RestPerpAssetCtxHyperliquid>,
);

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestPerpAssetCtxHyperliquid {
    #[serde(default)]
    pub markPx: Value,
    #[serde(default)]
    pub midPx: Value,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RestSpotMetaAndAssetCtxsHyperliquid(
    pub RestSpotMetaHyperliquid,
    pub Vec<RestSpotAssetCtxHyperliquid>,
);

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestSpotAssetCtxHyperliquid {
    #[serde(default)]
    pub markPx: Value,
    #[serde(default)]
    pub midPx: Value,
}
