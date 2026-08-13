use serde::Deserialize;

use crate::arch::market_assets::{
    base_data::{InstrumentType, MarginMode, PositionSide},
    exchange::okx::api_utils::{
        de_okx_inst, de_okx_instrument_type, de_okx_margin_mode, de_okx_position_side,
    },
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestSubPositionOkx {
    #[serde(deserialize_with = "de_okx_inst")]
    pub instId: String,
    pub subPosId: String,
    #[serde(deserialize_with = "de_okx_position_side")]
    pub posSide: PositionSide,
    #[serde(deserialize_with = "de_okx_margin_mode")]
    pub mgnMode: MarginMode,
    pub lever: String,
    pub openAvgPx: String,
    pub openTime: String,
    pub subPos: String,
    #[serde(deserialize_with = "de_okx_instrument_type")]
    pub instType: InstrumentType,
    pub margin: String,
    pub upl: String,
    pub uplRatio: String,
    pub markPx: Option<String>,
    pub uniqueCode: String,
    pub ccy: String,
}
