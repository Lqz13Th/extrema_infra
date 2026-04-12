use std::collections::HashMap;

use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::price_data::TickerData, base_data::InstrumentType,
    exchange::hyperliquid::api_utils::hyperliquid_perp_to_cli,
};

#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
pub struct RestAllMidsHyperliquid(pub HashMap<String, String>);

impl RestAllMidsHyperliquid {
    pub fn into_perp_ticker_data(self, timestamp: u64) -> Vec<TickerData> {
        self.0
            .into_iter()
            .filter(|(coin, _)| !coin.starts_with('@') && !coin.contains('/'))
            .map(|(coin, price)| TickerData {
                timestamp,
                inst: hyperliquid_perp_to_cli(&coin),
                inst_type: InstrumentType::Perpetual,
                price: price.parse().unwrap_or_default(),
            })
            .collect()
    }

    pub fn into_spot_ticker_data(
        self,
        timestamp: u64,
        spot_inst_by_coin: &HashMap<String, String>,
    ) -> Vec<TickerData> {
        self.0
            .into_iter()
            .filter_map(|(coin, price)| {
                let inst = spot_inst_by_coin.get(&coin)?.clone();

                Some(TickerData {
                    timestamp,
                    inst,
                    inst_type: InstrumentType::Spot,
                    price: price.parse().unwrap_or_default(),
                })
            })
            .collect()
    }
}
