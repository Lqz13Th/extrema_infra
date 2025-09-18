use serde::{Deserialize, Serialize};

use crate::market_assets::{
    api_general::get_micros_timestamp,
    utils_data::CurrentLeadtrader,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize, Serialize)]
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
            margin: d.margin.parse::<f64>().unwrap_or(0.0),
            copy_pnl: d.copyTotalPnl.parse::<f64>().unwrap_or(0.0),
            copy_amount: d.copyTotalAmt.parse::<f64>().unwrap_or(0.0),
        }
    }
}