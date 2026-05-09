use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestWithdrawHistoryBinance {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub amount: String,
    #[serde(default)]
    pub transactionFee: String,
    #[serde(default)]
    pub coin: String,
    #[serde(default)]
    pub status: i32,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub txId: String,
    #[serde(default)]
    pub applyTime: String,
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub transferType: i32,
    #[serde(default)]
    pub withdrawOrderId: String,
    #[serde(default)]
    pub info: String,
    #[serde(default)]
    pub confirmNo: i32,
    #[serde(default)]
    pub walletType: i32,
    #[serde(default)]
    pub completeTime: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
