use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::utils_data::InstrumentInfo,
    api_general::de_string_from_any,
    base_data::{InstrumentStatus, InstrumentType},
    exchange::gate::api_utils::gate_inst_to_cli,
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestDeliveryContractGate {
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub name: String,
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub order_price_round: String,
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub order_size_min: String,
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub order_size_max: String,
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub quanto_multiplier: String,
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub status: String,
    #[serde(default)]
    pub in_delisting: bool,
}

impl From<RestDeliveryContractGate> for InstrumentInfo {
    fn from(d: RestDeliveryContractGate) -> Self {
        let lot_size = d.order_size_min.parse().unwrap_or_default();
        let max_size = d.order_size_max.parse().unwrap_or_default();
        let tick_size = d.order_price_round.parse().unwrap_or_default();
        let state = if !d.status.is_empty() {
            match d.status.as_str() {
                "trading" => InstrumentStatus::Live,
                "delisting" => InstrumentStatus::Suspend,
                "delisted" => InstrumentStatus::Closed,
                _ => InstrumentStatus::Unknown,
            }
        } else if d.in_delisting {
            InstrumentStatus::Suspend
        } else {
            InstrumentStatus::Live
        };

        InstrumentInfo {
            inst: gate_inst_to_cli(&d.name),
            inst_code: None,
            inst_type: InstrumentType::Futures,
            lot_size,
            tick_size,
            min_lmt_size: lot_size,
            max_lmt_size: max_size,
            min_mkt_size: lot_size,
            max_mkt_size: max_size,
            contract_value: d.quanto_multiplier.parse().ok(),
            contract_multiplier: None,
            state,
        }
    }
}
