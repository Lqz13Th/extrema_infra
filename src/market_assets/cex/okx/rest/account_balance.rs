use serde::Deserialize;

use crate::market_assets::{
    api_data::account_data::BalanceData,
    api_general::ts_to_micros,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestAccountBalOkx {
    pub details: Vec<AccountBalDetails>
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
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
            timestamp: ts_to_micros(d.uTime.parse().unwrap_or_default()),
            asset: d.ccy,
            total: d.eq.parse().unwrap_or_default(),
            available: d.availBal.parse().unwrap_or_default(),
            frozen: d.frozenBal.parse().unwrap_or_default(),
        }
    }
}
