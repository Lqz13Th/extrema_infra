use serde::{Deserialize, Serialize};

use crate::market_assets::{
    api_general::{get_micros_timestamp, ts_to_micros},
    base_data::{InstrumentType, MarginMode, PositionSide},
    utils_data::PubLeadtrader,
    cex::okx::api_utils::okx_swap_to_cli,
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
    pub aum: String,                 // 带单规模，单位 USDT
    pub copyState: String,           // 当前跟单状态 0: 没在跟单, 1: 在跟单
    pub maxCopyTraderNum: String,    // 最大跟单人数
    pub copyTraderNum: String,       // 跟单人数
    pub accCopyTraderNum: String,    // 累计跟单人数
    pub portLink: String,            // 头像链接
    pub nickName: String,            // 昵称
    pub ccy: String,                 // 保证金币种
    pub uniqueCode: String,          // 交易员唯一标识码
    pub winRatio: String,            // 胜率，0.1 = 10%
    pub leadDays: String,            // 带单天数
    pub traderInsts: Vec<String>,    // 带单的合约列表
    pub pnl: String,                 // 近90日收益 USDT
    pub pnlRatio: String,            // 近90日收益率
    pub pnlRatios: Vec<PnlRatioData>,// 每日收益率数据
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PnlRatioData {
    pub beginTs: String,             // 当天收益率开始时间
    pub pnlRatio: String,            // 当天收益率
}

impl From<RestPubLeadTradersOkx> for Vec<PubLeadtrader> {
    fn from(d: RestPubLeadTradersOkx) -> Self {
        d.ranks.into_iter().map(|rank| {
            let latest_pnl_ratio = rank.pnlRatios.last()
                .map(|p| p.pnlRatio.parse::<f64>().unwrap_or(0.0))
                .unwrap_or(0.0);

            let timestamp = rank.pnlRatios.last()
                .map(|p| p.beginTs.parse::<u64>().unwrap_or(0))
                .unwrap_or(0);

            PubLeadtrader {
                timestamp,
                copytrader_id: rank.uniqueCode,
                nick_name: rank.nickName,
                margin: rank.aum.parse::<f64>().unwrap_or(0.0),
                copy_pnl: rank.pnl.parse::<f64>().unwrap_or(0.0),
                copy_amount: latest_pnl_ratio,
            }
        }).collect()
    }
}
