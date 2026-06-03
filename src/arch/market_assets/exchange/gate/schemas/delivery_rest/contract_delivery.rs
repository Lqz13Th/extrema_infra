use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::utils_data::InstrumentInfo,
    api_general::{de_opt_u64_from_string_or_number, de_string_from_any, get_seconds_timestamp},
    base_data::{InstrumentStatus, InstrumentType},
    exchange::gate::api_utils::gate_fut_inst_to_cli,
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestContractGateDelivery {
    #[serde(deserialize_with = "de_string_from_any")]
    pub name: String,
    #[serde(deserialize_with = "de_string_from_any")]
    pub order_price_round: String,
    #[serde(deserialize_with = "de_string_from_any")]
    pub order_size_min: String,
    #[serde(deserialize_with = "de_string_from_any")]
    pub order_size_max: String,
    #[serde(deserialize_with = "de_string_from_any")]
    pub quanto_multiplier: String,
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub status: String,
    pub in_delisting: bool,
    #[serde(default, deserialize_with = "de_opt_u64_from_string_or_number")]
    pub expire_time: Option<u64>,
}

impl RestContractGateDelivery {
    pub fn instrument_status(&self, now_secs: u64) -> InstrumentStatus {
        match self.status.as_str() {
            "delisted" => return InstrumentStatus::Closed,
            "delisting" => return InstrumentStatus::Delisting,
            _ => {},
        }

        if self
            .expire_time
            .is_some_and(|expire_time| expire_time > 0 && now_secs >= expire_time)
        {
            return InstrumentStatus::Closed;
        }

        if self.in_delisting {
            return InstrumentStatus::Delisting;
        }

        match self.status.as_str() {
            "" | "trading" => InstrumentStatus::Live,
            _ => InstrumentStatus::Unknown,
        }
    }

    pub fn is_live(&self, now_secs: u64) -> bool {
        self.instrument_status(now_secs) == InstrumentStatus::Live
    }
}

impl From<RestContractGateDelivery> for InstrumentInfo {
    fn from(d: RestContractGateDelivery) -> Self {
        let lot_size = d.order_size_min.parse().unwrap_or_default();
        let max_size = d.order_size_max.parse().unwrap_or_default();
        let tick_size = d.order_price_round.parse().unwrap_or_default();
        let state = d.instrument_status(get_seconds_timestamp());

        InstrumentInfo {
            inst: gate_fut_inst_to_cli(&d.name),
            inst_code: None,
            inst_type: InstrumentType::Futures,
            lot_size,
            tick_size,
            min_lmt_size: lot_size,
            max_lmt_size: max_size,
            min_mkt_size: lot_size,
            max_mkt_size: max_size,
            min_notional: None,
            contract_value: d.quanto_multiplier.parse().ok(),
            contract_multiplier: None,
            state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract(status: &str) -> RestContractGateDelivery {
        RestContractGateDelivery {
            name: "LTC_USDT_20260605".to_string(),
            order_price_round: "0.0001".to_string(),
            order_size_min: "1".to_string(),
            order_size_max: "100000".to_string(),
            quanto_multiplier: "1".to_string(),
            status: status.to_string(),
            in_delisting: false,
            expire_time: None,
        }
    }

    #[test]
    fn delivery_contract_before_expire_time_stays_live() {
        let mut c = contract("");
        c.expire_time = Some(1_780_646_400);

        assert_eq!(c.instrument_status(1_780_481_685), InstrumentStatus::Live);
        assert!(c.is_live(1_780_481_685));
    }

    #[test]
    fn delivery_contract_after_expire_time_marks_closed() {
        let mut c = contract("");
        c.expire_time = Some(1_780_646_400);

        assert_eq!(c.instrument_status(1_780_646_400), InstrumentStatus::Closed);
        assert!(!c.is_live(1_780_646_400));
    }

    #[test]
    fn delivery_contract_in_delisting_marks_delisting() {
        let mut c = contract("");
        c.in_delisting = true;
        c.expire_time = Some(1_780_646_400);

        assert_eq!(
            c.instrument_status(1_780_481_685),
            InstrumentStatus::Delisting
        );
        assert!(!c.is_live(1_780_481_685));
    }
}
