use std::collections::HashMap;

use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::account_data::BalanceData, api_general::get_micros_timestamp,
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestAccountBalGateUnified {
    pub balances: HashMap<String, GateUnifiedBalance>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GateUnifiedBalance {
    pub available: String,
    pub freeze: String,
    pub equity: String,
    pub borrowed: String,
}

impl From<RestAccountBalGateUnified> for Vec<BalanceData> {
    fn from(d: RestAccountBalGateUnified) -> Self {
        let timestamp = get_micros_timestamp();
        d.balances
            .into_iter()
            .map(|(asset, bal)| BalanceData {
                timestamp,
                asset,
                total: bal.equity.parse().unwrap_or_default(),
                available: bal.available.parse().unwrap_or_default(),
                frozen: bal.freeze.parse().unwrap_or_default(),
                borrowed: Some(bal.borrowed.parse().unwrap_or_default()),
            })
            .collect()
    }
}
