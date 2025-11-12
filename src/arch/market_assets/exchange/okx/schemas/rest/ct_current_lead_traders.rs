use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::utils_data::CurrentLeadtrader,
    api_general::get_micros_timestamp,
};

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
impl From<RestLeadtraderOkx> for CurrentLeadtrader {
    fn from(d: RestLeadtraderOkx) -> Self {
        CurrentLeadtrader {
            timestamp: get_micros_timestamp(),
            unique_code: d.uniqueCode,
            nick_name: d.nickName,
            margin: d.margin.parse().unwrap_or_default(),
            copy_pnl: d.copyTotalPnl.parse().unwrap_or_default(),
            copy_amount: d.copyTotalAmt.parse().unwrap_or_default(),
        }
    }
}