use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::account_data::OrderAckData, api_general::ts_to_micros, base_data::OrderStatus,
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestFuturesOrderGateFutures {
    pub id: i64,
    pub status: String,
    pub finish_as: Option<String>,
    pub update_time: f64,
    pub create_time: f64,
    pub text: Option<String>,
}

impl From<RestFuturesOrderGateFutures> for OrderAckData {
    fn from(d: RestFuturesOrderGateFutures) -> Self {
        let ts = if d.update_time > 0.0 {
            d.update_time
        } else {
            d.create_time
        };

        let status = match d.status.as_str() {
            "open" => OrderStatus::Live,
            "finished" => match d.finish_as.as_deref() {
                Some("filled") => OrderStatus::Filled,
                Some(
                    "cancelled" | "liquidated" | "ioc" | "auto_deleveraged" | "reduce_only"
                    | "position_closed" | "reduce_out" | "stp",
                ) => OrderStatus::Canceled,
                _ => OrderStatus::Unknown,
            },
            _ => OrderStatus::Unknown,
        };
        OrderAckData {
            timestamp: ts_to_micros(ts as u64),
            order_status: status,
            order_id: d.id.to_string(),
            cli_order_id: d.text,
        }
    }
}
