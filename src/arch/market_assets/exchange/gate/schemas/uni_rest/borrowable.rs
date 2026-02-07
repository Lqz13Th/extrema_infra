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

impl RestBorrowableGate {
    pub fn into_borrowable_data(self) -> BorrowableData {
        BorrowableData {
            timestamp: get_micros_timestamp(),
            asset: self.currency,
            available: self.amount.parse().unwrap_or_default(),
        }
    }
}
