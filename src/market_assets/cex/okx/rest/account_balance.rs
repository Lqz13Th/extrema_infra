use serde::Deserialize;

use crate::market_assets::account_data::BalanceData;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct AccountBalInfo {
    pub details: Vec<AccountBalDetails>
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct AccountBalDetails {
    pub ccy: String,
    pub eq: String,
    pub availBal : String,
    pub frozenBal : String,
    pub uTime: String,
}

impl From<AccountBalDetails> for BalanceData {
    fn from(d: AccountBalDetails) -> Self {
        BalanceData {
            asset: d.ccy,
            total: d.eq.parse().unwrap_or(0.0),
            available: d.availBal.parse().unwrap_or(0.0),
            frozen: d.frozenBal.parse().unwrap_or(0.0),
            timestamp: d.uTime.parse().unwrap_or(0),
        }
    }
}
