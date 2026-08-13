use serde::Deserialize;

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
