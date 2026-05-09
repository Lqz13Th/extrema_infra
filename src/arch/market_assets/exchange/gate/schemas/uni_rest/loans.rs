use serde::Deserialize;

use crate::arch::market_assets::api_general::de_micros_from_int;

#[derive(Clone, Debug, Deserialize)]
pub struct RestLoanTranGateUnified {
    pub tran_id: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RestLoanGateUnified {
    pub currency: String,
    #[serde(default, alias = "currency_pari")]
    pub currency_pair: String,
    pub amount: String,
    pub r#type: String,
    #[serde(deserialize_with = "de_micros_from_int")]
    pub create_time: u64,
    #[serde(alias = "change_time", deserialize_with = "de_micros_from_int")]
    pub update_time: u64,
}
