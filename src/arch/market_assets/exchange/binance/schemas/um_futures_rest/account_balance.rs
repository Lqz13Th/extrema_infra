use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::account_data::BalanceData,
    api_general::ts_to_micros,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestAccountBalBinanceUM {
    pub accountAlias: String,
    pub asset: String,
    pub balance: String,
    pub availableBalance: String,
    pub crossUnPnl: String,
    pub crossWalletBalance: String,
    pub maxWithdrawAmount: String,
    pub updateTime: u64,
}



impl From<RestAccountBalBinanceUM> for BalanceData {
    fn from(d: RestAccountBalBinanceUM) -> Self {

        let total= d.balance.parse::<f64>().unwrap_or_default();
        let available = d.availableBalance.parse::<f64>().unwrap_or_default();
        let cross_un_pnl = d.crossUnPnl.parse::<f64>().unwrap_or_default();
        let frozen = total - available - cross_un_pnl;

        BalanceData {
            timestamp: ts_to_micros(d.updateTime),
            asset: d.asset,
            total,
            available,
            frozen,
        }
    }
}
