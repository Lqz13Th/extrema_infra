use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::utils_data::InstrumentInfo,
    exchange::okx::api_utils::okx_inst_to_cli,
    base_data::{InstrumentStatus, InstrumentType},
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestInstrumentsOkx {
    pub instId: String,
    pub instType: String,
    pub lotSz: String,
    pub tickSz: String,
    pub minSz: String,
    pub maxLmtSz: String,
    pub maxMktSz: String,
    pub ctVal: Option<String>,
    pub ctMult: Option<String>,
    pub state: String,
    pub instIdCode: i64,
}

impl From<RestInstrumentsOkx> for InstrumentInfo {
    fn from(d: RestInstrumentsOkx) -> Self {
        InstrumentInfo {
            inst: okx_inst_to_cli(&d.instId),
            inst_code: Some(d.instIdCode.to_string()),
            inst_type: match d.instType.as_str() {
                "SWAP" => InstrumentType::Perpetual,
                "FUTURES" => InstrumentType::Futures,
                "SPOT" => InstrumentType::Spot,
                _ => InstrumentType::Unknown,
            },
            lot_size: d.lotSz.parse().unwrap_or_default(),
            tick_size: d.tickSz.parse().unwrap_or_default(),
            min_lmt_size: d.minSz.parse().unwrap_or_default(),
            max_lmt_size: d.maxLmtSz.parse().unwrap_or_default(),
            min_mkt_size: d.minSz.parse().unwrap_or_default(),
            max_mkt_size: d.maxMktSz.parse().unwrap_or_default(),
            contract_value: d.ctVal
                .as_ref()
                .and_then(|p| p.parse().ok()),
            contract_multiplier: d.ctMult
                .as_ref()
                .and_then(|p| p.parse().ok()),
            state: match d.state.as_str() {
                "live" => InstrumentStatus::Live,
                "suspend" => InstrumentStatus::Suspend,
                _ => InstrumentStatus::Unknown,
            },
        }
    }
}


