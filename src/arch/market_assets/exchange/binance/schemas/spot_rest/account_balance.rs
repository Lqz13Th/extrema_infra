use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::account_data::BalanceData, api_general::get_micros_timestamp,
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestAccountInfoBinanceSpot {
    pub balances: Vec<RestAccountBalBinanceSpot>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RestAccountBalBinanceSpot {
    pub asset: String,
    pub free: String,
    pub locked: String,
}

impl From<RestAccountBalBinanceSpot> for BalanceData {
    fn from(d: RestAccountBalBinanceSpot) -> Self {
        let available: f64 = d.free.parse().unwrap_or_default();
        let frozen: f64 = d.locked.parse().unwrap_or_default();

        BalanceData {
            timestamp: get_micros_timestamp(),
            asset: d.asset,
            total: available + frozen,
            available,
            frozen,
            borrowed: None,
        }
    }
}
