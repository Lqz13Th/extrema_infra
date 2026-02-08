use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::account_data::BorrowableData, api_general::get_micros_timestamp,
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestBorrowableGate {
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub amount: String,
}

impl From<RestBorrowableGate> for BorrowableData {
    fn from(data: RestBorrowableGate) -> Self {
        BorrowableData {
            timestamp: get_micros_timestamp(),
            asset: data.currency,
            available: data.amount.parse().unwrap_or_default(),
        }
    }
}
