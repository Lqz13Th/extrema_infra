use serde::Deserialize;

#[allow(non_snake_case)]
#[derive(Debug, Clone, Deserialize)]
pub struct RestSubPositionOkx {
    pub instId: String,
    pub subPosId: String,
    pub posSide: String,
    pub mgnMode: String,
    pub lever: String,
    pub openAvgPx: String,
    pub openTime: String,
    pub subPos: String,
    pub instType: String,
    pub margin: String,
    pub upl: String,
    pub uplRatio: String,
    pub markPx: Option<String>, // SPOT没有markPx
    pub uniqueCode: String,
    pub ccy: String,
}

