use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::utils_data::InstrumentInfo,
    base_data::{InstrumentStatus, InstrumentType},
    exchange::okx::api_utils::{okx_inst_to_cli, okx_preopen_inst},
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
    pub expTime: Option<String>,
    pub instIdCode: Option<i64>,
}

impl From<RestInstrumentsOkx> for InstrumentInfo {
    fn from(d: RestInstrumentsOkx) -> Self {
        let exp_time_present = d
            .expTime
            .as_deref()
            .map(str::trim)
            .is_some_and(|v| !v.is_empty() && v != "0");
        let preopen_inst = d
            .state
            .eq_ignore_ascii_case("preopen")
            .then(|| okx_preopen_inst(&d.instId))
            .flatten();
        let inst = okx_inst_to_cli(
            preopen_inst
                .as_ref()
                .map(|(_, inst)| inst.as_str())
                .unwrap_or(&d.instId),
        );
        let inst_type = preopen_inst
            .map(|(inst_type, _)| inst_type)
            .unwrap_or_else(|| match d.instType.as_str() {
                "SWAP" => InstrumentType::Perpetual,
                "FUTURES" => InstrumentType::Futures,
                "SPOT" => InstrumentType::Spot,
                _ => InstrumentType::Unknown,
            });

        InstrumentInfo {
            inst,
            inst_code: d.instIdCode.map(|x| x.to_string()),
            inst_type,
            lot_size: d.lotSz.parse().unwrap_or_default(),
            tick_size: d.tickSz.parse().unwrap_or_default(),
            min_lmt_size: d.minSz.parse().unwrap_or_default(),
            max_lmt_size: d.maxLmtSz.parse().unwrap_or_default(),
            min_mkt_size: d.minSz.parse().unwrap_or_default(),
            max_mkt_size: d.maxMktSz.parse().unwrap_or_default(),
            min_notional: None,
            contract_value: d.ctVal.and_then(|p| p.parse().ok()),
            contract_multiplier: d.ctMult.and_then(|p| p.parse().ok()),
            state: if exp_time_present {
                InstrumentStatus::Delisting
            } else {
                match d.state.as_str() {
                    "live" => InstrumentStatus::Live,
                    "suspend" => InstrumentStatus::Suspend,
                    "preopen" => InstrumentStatus::PreOpen,
                    _ => InstrumentStatus::Unknown,
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_okx_preopen_listing_instrument() {
        let info = InstrumentInfo::from(RestInstrumentsOkx {
            instId: "LISTING-SPOT-SLX-USDT".into(),
            instType: "SPOT".into(),
            lotSz: String::new(),
            tickSz: String::new(),
            minSz: String::new(),
            maxLmtSz: String::new(),
            maxMktSz: String::new(),
            ctVal: None,
            ctMult: None,
            state: "preopen".into(),
            expTime: None,
            instIdCode: None,
        });

        assert_eq!(info.inst, "SLX_USDT");
        assert_eq!(info.inst_type, InstrumentType::Spot);
        assert_eq!(info.state, InstrumentStatus::PreOpen);
    }
}
