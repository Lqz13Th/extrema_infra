use serde::Deserialize;

use crate::market_assets::api_data::utils_data::{PubLeadtrader, PubLeadtraderInfo};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestPubLeadTradersOkx {
    pub dataVer: String,
    pub totalPage: String,
    pub ranks: Vec<RankInfo>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
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
                aum: r.aum.parse().unwrap_or_default(),
                copy_state: r.copyState.parse().unwrap_or_default(),
                copy_trader_num: r.copyTraderNum.parse().unwrap_or_default(),
                max_copy_trader_num: r.maxCopyTraderNum.parse().unwrap_or_default(),
                accum_copy_trader_num: r.accCopyTraderNum.parse().unwrap_or_default(),
                lead_days: r.leadDays.parse().unwrap_or_default(),
                win_ratio: r.winRatio.parse().unwrap_or_default(),
                pnl_ratio: r.pnlRatio.parse().unwrap_or_default(),
                pnl: r.pnl.parse().unwrap_or_default(),
            }
        }).collect();

        PubLeadtraderInfo {
            data_version: d.dataVer.parse().unwrap_or_default(),
            total_page: d.totalPage.parse().unwrap_or_default(),
            pub_leadtraders: traders,
        }
    }
}
