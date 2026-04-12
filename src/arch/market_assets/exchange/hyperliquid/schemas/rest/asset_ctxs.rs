use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use crate::arch::market_assets::api_general::value_to_f64;
use crate::errors::{InfraError, InfraResult};

use super::{meta::RestMetaHyperliquid, spot_meta::RestSpotMetaHyperliquid};

#[derive(Clone, Debug, Deserialize)]
pub struct RestMetaAndAssetCtxsHyperliquid(
    pub RestMetaHyperliquid,
    pub Vec<RestPerpAssetCtxHyperliquid>,
);

impl RestMetaAndAssetCtxsHyperliquid {
    pub fn into_perp_mark_px_by_coin(self) -> InfraResult<HashMap<String, f64>> {
        let Self(meta, asset_ctxs) = self;
        if meta.universe.len() != asset_ctxs.len() {
            return Err(InfraError::ApiCliError(format!(
                "Hyperliquid metaAndAssetCtxs length mismatch: universe={}, ctxs={}",
                meta.universe.len(),
                asset_ctxs.len()
            )));
        }

        Ok(meta
            .universe
            .into_iter()
            .zip(asset_ctxs)
            .map(|(meta, ctx)| {
                let mark_price = match value_to_f64(&ctx.markPx) {
                    0.0 => value_to_f64(&ctx.midPx),
                    mark => mark,
                };
                (meta.name, mark_price)
            })
            .collect())
    }
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestPerpAssetCtxHyperliquid {
    #[serde(default)]
    pub funding: Value,
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
