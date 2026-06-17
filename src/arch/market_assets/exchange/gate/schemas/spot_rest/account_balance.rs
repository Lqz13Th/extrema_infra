use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::account_data::BalanceData, api_general::get_micros_timestamp,
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestAccountBalGateSpot {
    pub currency: String,
    pub available: String,
    pub locked: String,
}

impl From<RestAccountBalGateSpot> for BalanceData {
    fn from(d: RestAccountBalGateSpot) -> Self {
        let available = d.available.parse().unwrap_or_default();
        let frozen = d.locked.parse().unwrap_or_default();
        BalanceData {
            timestamp: get_micros_timestamp(),
            asset: d.currency,
            total: available + frozen,
            available,
            frozen,
            borrowed: None,
        }
    }
}
