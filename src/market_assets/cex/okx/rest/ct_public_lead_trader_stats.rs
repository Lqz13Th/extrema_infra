use serde::Deserialize;

use crate::market_assets::{
    api_data::utils_data::PubLeadtraderStats,
    api_general::get_micros_timestamp,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestPubLeadTraderStatsOkx {
    pub winRatio: String,
    pub profitDays: String,
    pub lossDays: String,
    pub curCopyTraderPnl: String,
    pub avgSubPosNotional: String,
    pub investAmt: String,
    pub ccy: String,
}

impl From<RestPubLeadTraderStatsOkx> for PubLeadtraderStats {
    fn from(d: RestPubLeadTraderStatsOkx) -> Self {
        PubLeadtraderStats {
            timestamp: get_micros_timestamp(),
            win_ratio: d.winRatio.parse().unwrap_or_default(),
            profit_days: d.profitDays.parse().unwrap_or_default(),
            loss_days: d.lossDays.parse().unwrap_or_default(),
            invest_amount: d.investAmt.parse().unwrap_or_default(),
            avg_sub_pos_national: d.avgSubPosNotional.parse().unwrap_or_default(),
            current_copy_trader_pnl: d.curCopyTraderPnl.parse().unwrap_or_default(),
        }
    }
}
