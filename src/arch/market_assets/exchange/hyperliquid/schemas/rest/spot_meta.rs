use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use crate::arch::market_assets::{
    api_data::utils_data::InstrumentInfo,
    base_data::{InstrumentStatus, InstrumentType},
    exchange::hyperliquid::api_utils::{hyperliquid_spot_asset_id, hyperliquid_spot_to_cli},
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestSpotMetaHyperliquid {
    pub universe: Vec<RestSpotUniverseHyperliquid>,
    pub tokens: Vec<RestSpotTokenHyperliquid>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestSpotUniverseHyperliquid {
    pub name: String,
    pub tokens: [u32; 2],
    pub index: u32,
    #[serde(default)]
    pub isCanonical: bool,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestSpotTokenHyperliquid {
    pub name: String,
    pub szDecimals: u32,
    pub index: u32,
    #[serde(default)]
    pub weiDecimals: Option<u32>,
    #[serde(default)]
    pub tokenId: Option<String>,
    #[serde(default)]
    pub isCanonical: Option<bool>,
    #[serde(default)]
    pub evmContract: Option<Value>,
    #[serde(default)]
    pub fullName: Option<String>,
}

impl RestSpotMetaHyperliquid {
    pub fn into_instrument_info(self) -> Vec<InstrumentInfo> {
        let token_map: HashMap<u32, RestSpotTokenHyperliquid> = self
            .tokens
            .into_iter()
            .map(|token| (token.index, token))
            .collect();

        self.universe
            .into_iter()
            .filter_map(|pair| pair.into_instrument_info(&token_map))
            .collect()
    }
}

impl RestSpotUniverseHyperliquid {
    fn into_instrument_info(
        self,
        token_map: &HashMap<u32, RestSpotTokenHyperliquid>,
    ) -> Option<InstrumentInfo> {
        let base = token_map.get(&self.tokens[0])?;
        let quote = token_map.get(&self.tokens[1])?;

        let lot_size = if base.szDecimals == 0 {
            1.0
        } else {
            10f64.powi(-(base.szDecimals as i32))
        };

        Some(InstrumentInfo {
            inst: hyperliquid_spot_to_cli(&self.name, &base.name, &quote.name),
            inst_code: Some(hyperliquid_spot_asset_id(self.index)),
            inst_type: InstrumentType::Spot,
            lot_size,
            tick_size: 0.0,
            min_lmt_size: lot_size,
            max_lmt_size: 0.0,
            min_mkt_size: lot_size,
            max_mkt_size: 0.0,
            min_notional: None,
            contract_value: None,
            contract_multiplier: None,
            state: InstrumentStatus::Live,
        })
    }
}
