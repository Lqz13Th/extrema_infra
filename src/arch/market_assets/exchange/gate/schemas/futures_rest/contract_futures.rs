use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::utils_data::{FundingRateData, FundingRateInfo, InstrumentInfo},
    api_general::{
        de_string_from_any, de_u64_from_string_or_number, get_micros_timestamp, ts_to_micros,
    },
    base_data::{InstrumentStatus, InstrumentType},
    exchange::gate::api_utils::gate_inst_to_cli,
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
    #[serde(deserialize_with = "de_string_from_any")]
    pub status: String,
    #[serde(deserialize_with = "de_string_from_any")]
    pub funding_rate: String,
    #[serde(deserialize_with = "de_u64_from_string_or_number")]
    pub funding_interval: u64,
    #[serde(deserialize_with = "de_u64_from_string_or_number")]
    pub funding_next_apply: u64,
}

impl RestContractGateFutures {
    pub fn into_funding_rate_data(self) -> FundingRateData {
        FundingRateData {
            timestamp: get_micros_timestamp(),
            inst: gate_inst_to_cli(&self.name),
            funding_rate: self.funding_rate.parse().unwrap_or_default(),
            funding_time: ts_to_micros(self.funding_next_apply),
        }
    }

    pub fn into_funding_rate_info(self) -> FundingRateInfo {
        FundingRateInfo {
            timestamp: get_micros_timestamp(),
            inst: gate_inst_to_cli(&self.name),
            funding_interval_sec: self.funding_interval as f64,
        }
    }
}

impl From<RestContractGateFutures> for InstrumentInfo {
    fn from(d: RestContractGateFutures) -> Self {
        let lot_size = d.order_size_min.parse().unwrap_or_default();
        let max_size = d.order_size_max.parse().unwrap_or_default();
        let tick_size = d.order_price_round.parse().unwrap_or_default();

        InstrumentInfo {
            inst: gate_inst_to_cli(&d.name),
            inst_code: None,
            inst_type: InstrumentType::Perpetual,
            lot_size,
            tick_size,
            min_lmt_size: lot_size,
            max_lmt_size: max_size,
            min_mkt_size: lot_size,
            max_mkt_size: max_size,
            contract_value: d.quanto_multiplier.parse().ok(),
            contract_multiplier: None,
            state: match d.status.as_str() {
                "trading" => InstrumentStatus::Live,
                "delisting" => InstrumentStatus::Suspend,
                "delisted" => InstrumentStatus::Closed,
                _ => InstrumentStatus::Unknown,
            },
        }
    }
}
