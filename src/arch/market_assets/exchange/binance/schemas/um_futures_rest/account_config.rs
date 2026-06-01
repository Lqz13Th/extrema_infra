use serde::Deserialize;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestAccountConfigBinanceUM {
    pub feeTier: u32,
    pub canTrade: bool,
    pub canDeposit: bool,
    pub canWithdraw: bool,
    pub dualSidePosition: bool,
    pub multiAssetsMargin: bool,
    pub tradeGroupId: i64,
    pub updateTime: u64,
}
