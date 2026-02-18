use serde::Deserialize;

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
    pub create_time: i64,
    #[serde(alias = "change_time")]
    pub update_time: i64,
}
