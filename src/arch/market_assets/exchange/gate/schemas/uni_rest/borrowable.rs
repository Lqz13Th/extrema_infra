use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::account_data::BorrowableData, api_general::get_micros_timestamp,
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestBorrowableGateUnified {
    pub currency: String,
    pub amount: String,
}

impl From<RestBorrowableGateUnified> for BorrowableData {
    fn from(d: RestBorrowableGateUnified) -> Self {
        BorrowableData {
            timestamp: get_micros_timestamp(),
            asset: d.currency,
            available: d.amount.parse().unwrap_or_default(),
        }
    }
}
