use serde::Deserialize;

use crate::arch::market_assets::exchange::binance::binance_rest_msg::BinanceCodeMsg;
use crate::arch::market_assets::{
    api_data::account_data::OrderAckData,
    api_general::{get_micros_timestamp, ts_to_micros},
    base_data::OrderStatus,
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
            msg: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum RestBatchOrderAckBinanceUM {
    Order(RestOrderAckBinanceUM),
    Error(BinanceCodeMsg),
}

impl RestBatchOrderAckBinanceUM {
    pub fn into_order_ack(
        self,
        order_id: Option<String>,
        cli_order_id: Option<String>,
    ) -> OrderAckData {
        match self {
            Self::Order(order) => order.into(),
            Self::Error(error) => OrderAckData {
                timestamp: get_micros_timestamp(),
                order_status: OrderStatus::Rejected,
                order_id: order_id.unwrap_or_default(),
                cli_order_id,
                msg: Some(format!(
                    "Binance REST error (code={}): {}",
                    error.code, error.msg
                )),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_batch_success_and_rejection() {
        let response: Vec<RestBatchOrderAckBinanceUM> = serde_json::from_str(
            r#"[{"clientOrderId":"one","orderId":42,"status":"NEW","updateTime":1000},{"code":-2010,"msg":"Order rejected"}]"#,
        )
        .unwrap();

        let mut response = response.into_iter();
        let success = response.next().unwrap().into_order_ack(None, None);
        assert_eq!(success.order_status, OrderStatus::Live);
        assert_eq!(success.order_id, "42");

        let rejected = response
            .next()
            .unwrap()
            .into_order_ack(None, Some("two".into()));
        assert_eq!(rejected.order_status, OrderStatus::Rejected);
        assert_eq!(rejected.cli_order_id.as_deref(), Some("two"));
        assert_eq!(
            rejected.msg.as_deref(),
            Some("Binance REST error (code=-2010): Order rejected")
        );
    }
}
