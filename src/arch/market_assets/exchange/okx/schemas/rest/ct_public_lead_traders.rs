use serde::Deserialize;

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
    pub aum: String,              // Assets under management (AUM), unit: USDT
    pub copyState: String,        // Current copy-trading state: 0 = not copying, 1 = copying
    pub maxCopyTraderNum: String, // Maximum number of copy traders allowed
    pub copyTraderNum: String,    // Current number of copy traders
    pub accCopyTraderNum: String, // Accumulated number of copy traders
    pub portLink: String,         // Profile image link
    pub nickName: String,         // Trader's nickname
    pub ccy: String,              // Margin currency
    pub uniqueCode: String,       // Unique identifier of the trader
    pub winRatio: String,         // Win ratio, e.g., 0.1 = 10%
    pub leadDays: String,         // Number of lead-trading days
    pub pnl: String,              // Profit and loss in the past 90 days (unit: USDT)
    pub pnlRatio: String,         // Profit and loss ratio in the past 90 days
}
