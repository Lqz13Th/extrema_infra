use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::account_data::OrderAckData, api_general::ts_to_micros, base_data::OrderStatus,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestOrderAckBinanceUM {
    pub clientOrderId: Option<String>,
    pub orderId: u64,
    pub status: String,
    pub updateTime: u64,
}

impl From<RestOrderAckBinanceUM> for OrderAckData {
    fn from(d: RestOrderAckBinanceUM) -> Self {
        OrderAckData {
            timestamp: ts_to_micros(d.updateTime),
            order_status: match d.status.as_str() {
                "NEW" => OrderStatus::Live,
                "PARTIALLY_FILLED" => OrderStatus::PartiallyFilled,
                "FILLED" => OrderStatus::Filled,
                "CANCELED" => OrderStatus::Canceled,
                "REJECTED" => OrderStatus::Rejected,
                "EXPIRED" => OrderStatus::Expired,
                _ => OrderStatus::Unknown,
            },
            order_id: d.orderId.to_string(),
            cli_order_id: d.clientOrderId,
        }
    }
}
