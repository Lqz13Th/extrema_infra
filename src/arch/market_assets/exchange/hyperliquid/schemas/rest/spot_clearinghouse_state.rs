use serde::Deserialize;
use serde_json::Value;

use crate::arch::market_assets::{
    api_data::account_data::BalanceData, api_general::value_to_f64,
    exchange::hyperliquid::api_utils::hyperliquid_symbol_to_cli_symbol,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestSpotClearinghouseStateHyperliquid {
    pub balances: Vec<RestSpotBalanceHyperliquid>,
}

impl RestSpotClearinghouseStateHyperliquid {
    pub fn into_balance_data(self) -> Vec<BalanceData> {
        self.balances.into_iter().map(BalanceData::from).collect()
    }
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestSpotBalanceHyperliquid {
    pub coin: String,
    #[serde(default)]
    pub hold: Value,
    #[serde(default)]
    pub total: Value,
}

impl From<RestSpotBalanceHyperliquid> for BalanceData {
    fn from(d: RestSpotBalanceHyperliquid) -> Self {
        let total = value_to_f64(&d.total);
        let frozen = value_to_f64(&d.hold);

        BalanceData {
            timestamp: 0,
            asset: hyperliquid_symbol_to_cli_symbol(&d.coin),
            total,
            frozen,
            available: (total - frozen).max(0.0),
            borrowed: None,
        }
    }
}
