use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::utils_data::InstrumentInfo,
    base_data::{InstrumentStatus, InstrumentType},
    exchange::hyperliquid::api_utils::{hyperliquid_perp_asset_id, hyperliquid_perp_to_cli},
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestMetaHyperliquid {
    pub universe: Vec<RestMetaUniverseHyperliquid>,
    #[serde(default, rename = "collateralToken")]
    pub collateral_token: Option<u32>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestMetaUniverseHyperliquid {
    pub name: String,
    pub szDecimals: u32,
    pub maxLeverage: Option<u32>,
    pub onlyIsolated: Option<bool>,
    pub isDelisted: Option<bool>,
    pub marginMode: Option<String>,
    #[serde(default)]
    pub growthMode: Option<String>,
    #[serde(default)]
    pub deployerFeeScale: Option<String>,
    #[serde(default)]
    pub lastFeeScaleChangeTime: Option<String>,
}

impl RestMetaHyperliquid {
    pub fn into_instrument_info(self, quote: &str) -> Vec<InstrumentInfo> {
        self.universe
            .into_iter()
            .enumerate()
            .map(|(index, inst)| inst.into_instrument_info(index, quote))
            .collect()
    }
}

impl RestMetaUniverseHyperliquid {
    fn into_instrument_info(self, index: usize, quote: &str) -> InstrumentInfo {
        let lot_size = if self.szDecimals == 0 {
            1.0
        } else {
            10f64.powi(-(self.szDecimals as i32))
        };

        InstrumentInfo {
            inst: hyperliquid_perp_to_cli(&self.name, quote),
            inst_code: Some(hyperliquid_perp_asset_id(index)),
            inst_type: InstrumentType::Perpetual,
            lot_size,
            tick_size: 0.0,
            min_lmt_size: lot_size,
            max_lmt_size: f64::MAX,
            min_mkt_size: lot_size,
            max_mkt_size: f64::MAX,
            max_leverage: self.maxLeverage,
            min_notional: None,
            contract_value: None,
            contract_multiplier: None,
            state: if self.isDelisted.unwrap_or(false) {
                InstrumentStatus::Closed
            } else {
                InstrumentStatus::Live
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hip3_fee_metadata() {
        let meta: RestMetaHyperliquid = serde_json::from_value(serde_json::json!({
            "universe": [{
                "name": "xyz:XYZ100",
                "szDecimals": 3,
                "maxLeverage": 20,
                "growthMode": "enabled",
                "deployerFeeScale": "1.0",
                "lastFeeScaleChangeTime": "2025-11-23T17:37:10.033211662"
            }]
        }))
        .unwrap();

        let xyz100 = &meta.universe[0];
        assert_eq!(xyz100.growthMode.as_deref(), Some("enabled"));
        assert_eq!(xyz100.deployerFeeScale.as_deref(), Some("1.0"));
        assert_eq!(
            xyz100.lastFeeScaleChangeTime.as_deref(),
            Some("2025-11-23T17:37:10.033211662")
        );
    }
}
