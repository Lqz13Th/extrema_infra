use serde::Deserialize;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestLeadtraderOkx {
    pub beginCopyTime: String,
    pub ccy: String,
    pub copyTotalAmt: String,
    pub copyTotalPnl: String,
    pub leadMode: String,
    pub margin: String,
    pub nickName: String,
    pub portLink: String,
    pub profitSharingRatio: String,
    pub todayPnl: String,
    pub uniqueCode: String,
    pub upl: String,
}
