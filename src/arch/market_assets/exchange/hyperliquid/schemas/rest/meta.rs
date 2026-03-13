use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::utils_data::InstrumentInfo,
    base_data::{InstrumentStatus, InstrumentType},
    exchange::hyperliquid::api_utils::{hyperliquid_perp_asset_id, hyperliquid_perp_to_cli},
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestMetaHyperliquid {
    pub universe: Vec<RestMetaUniverseHyperliquid>,
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
}

impl RestMetaHyperliquid {
    pub fn into_instrument_info(self) -> Vec<InstrumentInfo> {
        self.universe
            .into_iter()
            .enumerate()
            .map(|(index, inst)| inst.into_instrument_info(index))
            .collect()
    }
}

impl RestMetaUniverseHyperliquid {
    fn into_instrument_info(self, index: usize) -> InstrumentInfo {
        let lot_size = if self.szDecimals == 0 {
            1.0
        } else {
            10f64.powi(-(self.szDecimals as i32))
        };

        InstrumentInfo {
            inst: hyperliquid_perp_to_cli(&self.name),
            inst_code: Some(hyperliquid_perp_asset_id(index)),
            inst_type: InstrumentType::Perpetual,
            lot_size,
            tick_size: 0.0,
            min_lmt_size: lot_size,
            max_lmt_size: 0.0,
            min_mkt_size: lot_size,
            max_mkt_size: 0.0,
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
