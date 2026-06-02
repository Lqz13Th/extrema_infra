use serde::Deserialize;

use crate::arch::market_assets::api_general::{de_micros_from_int, de_string_from_any};

#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestSubAccountTransferGate {
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub tx_id: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestSubAccountTransferHistoryGate {
    #[serde(default, deserialize_with = "de_micros_from_int")]
    pub timest: u64,
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub uid: String,
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub sub_account: String,
    #[serde(default)]
    pub sub_account_type: String,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub amount: String,
    #[serde(default)]
    pub direction: String,
    #[serde(default)]
    pub source: String,
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub client_order_id: String,
    #[serde(default)]
    pub status: String,
}
