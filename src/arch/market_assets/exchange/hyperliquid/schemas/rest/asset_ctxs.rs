use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use crate::arch::market_assets::{
    api_data::utils_data::{FundingRateData, FundingRateInfo},
    api_general::{get_mills_timestamp, ts_to_micros, value_to_f64},
    exchange::hyperliquid::api_utils::{
        hyperliquid_funding_interval_sec, hyperliquid_next_funding_time_ms, hyperliquid_perp_to_cli,
    },
};
use crate::errors::{InfraError, InfraResult};

use super::{meta::RestMetaHyperliquid, spot_meta::RestSpotMetaHyperliquid};

#[derive(Clone, Debug, Deserialize)]
pub struct RestMetaAndAssetCtxsHyperliquid(
    pub RestMetaHyperliquid,
    pub Vec<RestPerpAssetCtxHyperliquid>,
);

impl RestMetaAndAssetCtxsHyperliquid {
    fn split(self) -> InfraResult<(RestMetaHyperliquid, Vec<RestPerpAssetCtxHyperliquid>)> {
        let Self(meta, asset_ctxs) = self;
        if meta.universe.len() != asset_ctxs.len() {
            return Err(InfraError::ApiCliError(format!(
                "Hyperliquid metaAndAssetCtxs length mismatch: universe={}, ctxs={}",
                meta.universe.len(),
                asset_ctxs.len()
            )));
        }

        Ok((meta, asset_ctxs))
    }

    pub fn into_perp_mark_px_by_coin(self) -> InfraResult<HashMap<String, f64>> {
        let (meta, asset_ctxs) = self.split()?;

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

    pub fn into_funding_rate_data(self) -> InfraResult<Vec<FundingRateData>> {
        let now_ms = get_mills_timestamp();
        let timestamp = ts_to_micros(now_ms);
        let funding_time = ts_to_micros(hyperliquid_next_funding_time_ms(now_ms));
        let (meta, asset_ctxs) = self.split()?;

        Ok(meta
            .universe
            .into_iter()
            .zip(asset_ctxs)
            .map(|(meta, ctx)| FundingRateData {
                timestamp,
                inst: hyperliquid_perp_to_cli(&meta.name),
                funding_rate: value_to_f64(&ctx.funding),
                funding_time,
            })
            .collect())
    }

    pub fn into_funding_rate_info(self) -> InfraResult<Vec<FundingRateInfo>> {
        let timestamp = ts_to_micros(get_mills_timestamp());
        let funding_interval_sec = hyperliquid_funding_interval_sec();
        let (meta, asset_ctxs) = self.split()?;

        Ok(meta
            .universe
            .into_iter()
            .zip(asset_ctxs)
            .map(|(meta, _)| FundingRateInfo {
                timestamp,
                inst: hyperliquid_perp_to_cli(&meta.name),
                funding_interval_sec,
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
