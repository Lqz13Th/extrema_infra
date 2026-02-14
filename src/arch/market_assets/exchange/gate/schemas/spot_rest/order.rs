use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::account_data::OrderAckData, api_general::ts_to_micros, base_data::OrderStatus,
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestOrderGateSpot {
    pub id: String,
    pub status: String,
    pub update_time_ms: u64,
    pub create_time_ms: u64,
    pub text: Option<String>,
}

impl From<RestOrderGateSpot> for OrderAckData {
    fn from(d: RestOrderGateSpot) -> Self {
        let ts = if d.update_time_ms > 0 {
            d.update_time_ms
        } else {
            d.create_time_ms
        };
        OrderAckData {
            timestamp: ts_to_micros(ts),
            order_status: match d.status.as_str() {
                "open" => OrderStatus::Live,
                "closed" => OrderStatus::Filled,
                "cancelled" => OrderStatus::Canceled,
                _ => OrderStatus::Unknown,
            },
            order_id: d.id,
            cli_order_id: d.text,
        }
    }
}
