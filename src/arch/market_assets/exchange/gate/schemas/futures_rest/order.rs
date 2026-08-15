use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::account_data::OrderAckData,
    api_general::{de_string_from_any, get_micros_timestamp, ts_to_micros},
    base_data::OrderStatus,
};

fn gate_futures_order_status(status: &str, finish_as: Option<&str>) -> OrderStatus {
    match status {
        "open" => OrderStatus::Live,
        "finished" => match finish_as {
            Some("filled") => OrderStatus::Filled,
            Some(
                "cancelled" | "liquidated" | "ioc" | "auto_deleveraged" | "reduce_only"
                | "position_closed" | "reduce_out" | "stp",
            ) => OrderStatus::Canceled,
            _ => OrderStatus::Unknown,
        },
        _ => OrderStatus::Unknown,
    }
}

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

        let status = gate_futures_order_status(&d.status, d.finish_as.as_deref());
        let msg = d.finish_as.clone().filter(|value| !value.is_empty());

        OrderAckData {
            timestamp: ts_to_micros(ts as u64),
            order_status: status,
            order_id: d.id.to_string(),
            cli_order_id: d.text,
            msg,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct RestBatchOrderGateFutures {
    #[serde(default)]
    pub succeeded: bool,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub id: String,
    #[serde(default)]
    pub status: String,
    pub finish_as: Option<String>,
    #[serde(default)]
    pub update_time: f64,
    #[serde(default)]
    pub create_time: f64,
    pub text: Option<String>,
}

impl RestBatchOrderGateFutures {
    pub fn into_order_ack(self, client_order_id: Option<String>) -> OrderAckData {
        let timestamp = if self.update_time > 0.0 {
            ts_to_micros(self.update_time as u64)
        } else if self.create_time > 0.0 {
            ts_to_micros(self.create_time as u64)
        } else {
            get_micros_timestamp()
        };
        let msg = if self.succeeded {
            self.finish_as.clone().filter(|value| !value.is_empty())
        } else {
            match (self.label.is_empty(), self.detail.is_empty()) {
                (false, false) => Some(format!("{}: {}", self.label, self.detail)),
                (false, true) => Some(self.label),
                (true, false) => Some(self.detail),
                (true, true) => None,
            }
        };

        OrderAckData {
            timestamp,
            order_status: if self.succeeded {
                gate_futures_order_status(&self.status, self.finish_as.as_deref())
            } else {
                OrderStatus::Rejected
            },
            order_id: self.id,
            cli_order_id: self.text.or(client_order_id),
            msg,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct RestBatchCancelOrderGateFutures {
    #[serde(default, deserialize_with = "de_string_from_any")]
    pub id: String,
    #[serde(default)]
    pub succeeded: bool,
    #[serde(default)]
    pub message: String,
}

impl RestBatchCancelOrderGateFutures {
    pub fn into_order_ack(self, client_order_id: Option<String>) -> OrderAckData {
        OrderAckData {
            timestamp: get_micros_timestamp(),
            order_status: if self.succeeded {
                OrderStatus::Canceled
            } else {
                OrderStatus::Rejected
            },
            order_id: self.id,
            cli_order_id: client_order_id,
            msg: (!self.message.is_empty()).then_some(self.message),
        }
    }
}
