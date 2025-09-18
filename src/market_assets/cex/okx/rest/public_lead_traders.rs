use serde::{Deserialize, Serialize};

use crate::market_assets::{
    utils_data::{PubLeadtraderInfo, PubLeadtrader},
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RestPubLeadTradersOkx {
    pub dataVer: String,
    pub totalPage: String,
    pub ranks: Vec<RankInfo>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RankInfo {
    pub aum: String,                 // Assets under management (AUM), unit: USDT
    pub copyState: String,           // Current copy-trading state: 0 = not copying, 1 = copying
    pub maxCopyTraderNum: String,    // Maximum number of copy traders allowed
    pub copyTraderNum: String,       // Current number of copy traders
    pub accCopyTraderNum: String,    // Accumulated number of copy traders
    pub portLink: String,            // Profile image link
    pub nickName: String,            // Trader's nickname
    pub ccy: String,                 // Margin currency
    pub uniqueCode: String,          // Unique identifier of the trader
    pub winRatio: String,            // Win ratio, e.g., 0.1 = 10%
    pub leadDays: String,            // Number of lead-trading days
    pub pnl: String,                 // Profit and loss in the past 90 days (unit: USDT)
    pub pnlRatio: String,            // Profit and loss ratio in the past 90 days
}

impl From<RestPubLeadTradersOkx> for PubLeadtraderInfo {
    fn from(d: RestPubLeadTradersOkx) -> Self {
        let traders = d.ranks.into_iter().map(|r| {
            PubLeadtrader {
                unique_code: r.uniqueCode,
                nick_name: r.nickName,
                aum: r.aum.parse::<f64>().unwrap_or(0.0),
                copy_state: r.copyState.parse::<u64>().unwrap_or(0),
                copy_trader_num: r.copyTraderNum.parse::<u64>().unwrap_or(0),
                max_copy_trader_num: r.maxCopyTraderNum.parse::<u64>().unwrap_or(0),
                accum_copy_trader_num: r.accCopyTraderNum.parse::<u64>().unwrap_or(0),
                lead_days: r.leadDays.parse::<u64>().unwrap_or(0),
                win_ratio: r.winRatio.parse::<f64>().unwrap_or(0.0),
                pnl_ratio: r.pnlRatio.parse::<f64>().unwrap_or(0.0),
                pnl: r.pnl.parse::<f64>().unwrap_or(0.0),
            }
        }).collect();

        PubLeadtraderInfo {
            data_version: d.dataVer.parse::<u64>().unwrap_or(0),
            total_page: d.totalPage.parse::<u64>().unwrap_or(0),
            pub_leadtraders: traders,
        }
    }
}
