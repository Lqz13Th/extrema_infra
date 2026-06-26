use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::utils_data::{FundingRateData, FundingRateInfo, InstrumentInfo},
    api_general::{
        de_opt_u64_from_string_or_number, de_string_from_any, de_u64_from_string_or_number,
        get_micros_timestamp, get_seconds_timestamp, ts_to_micros,
    },
    base_data::{InstrumentStatus, InstrumentType},
    exchange::gate::api_utils::gate_fut_inst_to_cli,
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestContractGateFutures {
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
    #[serde(default)]
    pub enable_decimal: bool,
    #[serde(deserialize_with = "de_string_from_any")]
    pub status: String,
    #[serde(default)]
    pub in_delisting: bool,
    #[serde(default, deserialize_with = "de_opt_u64_from_string_or_number")]
    pub delisting_time: Option<u64>,
    #[serde(default, deserialize_with = "de_opt_u64_from_string_or_number")]
    pub delisted_time: Option<u64>,
    #[serde(deserialize_with = "de_string_from_any")]
    pub funding_rate: String,
    #[serde(deserialize_with = "de_u64_from_string_or_number")]
    pub funding_interval: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_number")]
    pub funding_next_apply: u64,
}

impl RestContractGateFutures {
    pub fn instrument_status(&self, now_secs: u64) -> InstrumentStatus {
        match self.status.as_str() {
            "delisted" => return InstrumentStatus::Closed,
            "delisting" => return InstrumentStatus::Delisting,
            _ => {},
        }

        if self
            .delisted_time
            .is_some_and(|delisted_time| delisted_time > 0 && now_secs >= delisted_time)
        {
            return InstrumentStatus::Closed;
        }

        if self.in_delisting
            || self
                .delisting_time
                .is_some_and(|delisting_time| delisting_time > 0)
        {
            return InstrumentStatus::Delisting;
        }

        match self.status.as_str() {
            "trading" => InstrumentStatus::Live,
            _ => InstrumentStatus::Unknown,
        }
    }

    pub fn is_live(&self, now_secs: u64) -> bool {
        self.instrument_status(now_secs) == InstrumentStatus::Live
    }
}

impl From<RestContractGateFutures> for InstrumentInfo {
    fn from(d: RestContractGateFutures) -> Self {
        let lot_size = if d.enable_decimal {
            0.0
        } else {
            d.order_size_min.parse().unwrap_or_default()
        };
        let min_size = if d.enable_decimal { 0.1 } else { lot_size };
        let max_size = d.order_size_max.parse().unwrap_or_default();
        let tick_size = d.order_price_round.parse().unwrap_or_default();
        let state = d.instrument_status(get_seconds_timestamp());

        InstrumentInfo {
            inst: gate_fut_inst_to_cli(&d.name),
            inst_code: None,
            inst_type: InstrumentType::Perpetual,
            lot_size,
            tick_size,
            min_lmt_size: min_size,
            max_lmt_size: max_size,
            min_mkt_size: min_size,
            max_mkt_size: max_size,
            min_notional: None,
            contract_value: d.quanto_multiplier.parse().ok(),
            contract_multiplier: None,
            state,
        }
    }
}

impl From<RestContractGateFutures> for FundingRateData {
    fn from(d: RestContractGateFutures) -> Self {
        FundingRateData {
            timestamp: get_micros_timestamp(),
            inst: gate_fut_inst_to_cli(&d.name),
            funding_rate: d.funding_rate.parse().unwrap_or_default(),
            funding_time: ts_to_micros(d.funding_next_apply),
        }
    }
}

impl From<RestContractGateFutures> for FundingRateInfo {
    fn from(d: RestContractGateFutures) -> Self {
        FundingRateInfo {
            timestamp: get_micros_timestamp(),
            inst: gate_fut_inst_to_cli(&d.name),
            funding_interval_sec: d.funding_interval as f64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract(status: &str) -> RestContractGateFutures {
        RestContractGateFutures {
            name: "DEGO_USDT".to_string(),
            order_price_round: "0.0001".to_string(),
            order_size_min: "1".to_string(),
            order_size_max: "680000".to_string(),
            quanto_multiplier: "0.1".to_string(),
            enable_decimal: false,
            status: status.to_string(),
            in_delisting: false,
            delisting_time: None,
            delisted_time: None,
            funding_rate: "0".to_string(),
            funding_interval: 28_800,
            funding_next_apply: 0,
        }
    }

    #[test]
    fn scheduled_delisting_time_marks_contract_as_delisting() {
        let mut c = contract("trading");
        c.delisting_time = Some(1_780_644_600);
        c.delisted_time = Some(1_780_646_400);

        assert_eq!(
            c.instrument_status(1_780_481_685),
            InstrumentStatus::Delisting
        );
        assert!(!c.is_live(1_780_481_685));
    }

    #[test]
    fn delisted_time_after_now_marks_contract_as_closed() {
        let mut c = contract("trading");
        c.delisting_time = Some(1_780_644_600);
        c.delisted_time = Some(1_780_646_400);

        assert_eq!(c.instrument_status(1_780_646_400), InstrumentStatus::Closed);
    }

    #[test]
    fn trading_without_delisting_time_stays_live() {
        let c = contract("trading");

        assert_eq!(c.instrument_status(1_780_481_685), InstrumentStatus::Live);
        assert!(c.is_live(1_780_481_685));
    }

    #[test]
    fn non_decimal_contract_uses_order_size_min_as_lot_and_min_size() {
        let info = InstrumentInfo::from(contract("trading"));

        assert_eq!(info.lot_size, 1.0);
        assert_eq!(info.min_lmt_size, 1.0);
        assert_eq!(info.min_mkt_size, 1.0);
        assert_eq!(info.contract_value, Some(0.1));
    }

    #[test]
    fn decimal_contract_uses_hidden_min_without_fake_lot_step() {
        let mut c = contract("trading");
        c.order_size_min = "0".to_string();
        c.quanto_multiplier = "100".to_string();
        c.enable_decimal = true;

        let info = InstrumentInfo::from(c);

        assert_eq!(info.lot_size, 0.0);
        assert_eq!(info.min_lmt_size, 0.1);
        assert_eq!(info.min_mkt_size, 0.1);
        assert_eq!(info.contract_value, Some(100.0));
    }
}
