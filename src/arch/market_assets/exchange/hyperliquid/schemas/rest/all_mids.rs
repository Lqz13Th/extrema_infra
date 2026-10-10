use std::collections::HashMap;

use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::price_data::TickerData,
    base_data::InstrumentType,
    exchange::hyperliquid::api_utils::{hyperliquid_is_spot_coin, hyperliquid_perp_to_cli},
};

#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
pub struct RestAllMidsHyperliquid(pub HashMap<String, String>);

impl RestAllMidsHyperliquid {
    pub fn into_perp_ticker_data(self, timestamp: u64, quote: &str) -> Vec<TickerData> {
        self.0
            .into_iter()
            .filter(|(coin, _)| !hyperliquid_is_spot_coin(coin))
            .map(|(coin, price)| TickerData {
                timestamp,
                inst: hyperliquid_perp_to_cli(&coin, quote),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perp_tickers_skip_spot_and_outcome_coins() {
        let mids = RestAllMidsHyperliquid(HashMap::from([
            ("BTC".to_string(), "63900.1".to_string()),
            ("@1".to_string(), "12.011".to_string()),
            ("#102220".to_string(), "0.500025".to_string()),
            ("PURR/USDC".to_string(), "0.115805".to_string()),
        ]));

        let tickers = mids.into_perp_ticker_data(0, "USDC");

        assert_eq!(tickers.len(), 1);
        assert_eq!(tickers[0].inst, "BTC_USDC_PERP");
    }
}
