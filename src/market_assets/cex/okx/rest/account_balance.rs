use serde::Deserialize;

use crate::market_assets::account_data::BalanceData;
use crate::market_assets::api_general::ts_to_micros;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestAccountBalOkx {
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
            timestamp: ts_to_micros(d.uTime.parse().unwrap_or(0)),
            asset: d.ccy,
            total: d.eq.parse().unwrap_or(0.0),
            available: d.availBal.parse().unwrap_or(0.0),
            frozen: d.frozenBal.parse().unwrap_or(0.0),
        }
    }
}
