use std::collections::HashMap;

use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::account_data::BalanceData, api_general::get_micros_timestamp,
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestAccountBalGate {
    pub balances: HashMap<String, GateUnifiedBalance>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GateUnifiedBalance {
    #[serde(default)]
    pub available: String,
    #[serde(default)]
    pub freeze: String,
    #[serde(default)]
    pub equity: String,
}

impl From<RestAccountBalGate> for Vec<BalanceData> {
    fn from(data: RestAccountBalGate) -> Self {
        let timestamp = get_micros_timestamp();
        data.balances
            .into_iter()
            .map(|(asset, bal)| BalanceData {
                timestamp,
                asset,
                total: bal.equity.parse().unwrap_or_default(),
                available: bal.available.parse().unwrap_or_default(),
                frozen: bal.freeze.parse().unwrap_or_default(),
            })
            .collect()
    }
}
