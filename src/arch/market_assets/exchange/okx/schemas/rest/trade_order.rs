use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::account_data::OrderAckData, api_general::ts_to_micros, base_data::OrderStatus,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestOrderAckOkx {
    pub clOrdId: Option<String>,
    pub ordId: String,
    pub tag: Option<String>,
    pub ts: String,
    pub sCode: String,
    pub sMsg: String,
}

impl From<RestOrderAckOkx> for OrderAckData {
    fn from(d: RestOrderAckOkx) -> Self {
        let msg = if d.sMsg.is_empty() {
            None
        } else {
            Some(d.sMsg.clone())
        };

        OrderAckData {
            timestamp: ts_to_micros(d.ts.parse().unwrap_or_default()),
            order_status: if d.sCode == "0" {
                OrderStatus::Live
            } else {
                OrderStatus::Rejected
            },
            order_id: d.ordId,
            cli_order_id: d.clOrdId,
            msg,
        }
    }
}

impl RestOrderAckOkx {
    pub fn into_cancel_ack(self) -> OrderAckData {
        let msg = (!self.sMsg.is_empty()).then_some(self.sMsg);

        OrderAckData {
            timestamp: ts_to_micros(self.ts.parse().unwrap_or_default()),
            order_status: if self.sCode == "0" {
                OrderStatus::Canceled
            } else {
                OrderStatus::Rejected
            },
            order_id: self.ordId,
            cli_order_id: self.clOrdId,
            msg,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_ack_maps_success_and_rejection() {
        let success: RestOrderAckOkx = serde_json::from_str(
            r#"{"clOrdId":"cancel1","ordId":"42","tag":"","ts":"1000","sCode":"0","sMsg":""}"#,
        )
        .unwrap();
        let success = success.into_cancel_ack();
        assert_eq!(success.order_status, OrderStatus::Canceled);
        assert_eq!(success.order_id, "42");

        let rejected: RestOrderAckOkx = serde_json::from_str(
            r#"{"clOrdId":"cancel2","ordId":"43","tag":"","ts":"1001","sCode":"51400","sMsg":"Order cancellation failed"}"#,
        )
        .unwrap();
        let rejected = rejected.into_cancel_ack();
        assert_eq!(rejected.order_status, OrderStatus::Rejected);
        assert_eq!(rejected.msg.as_deref(), Some("Order cancellation failed"));
    }
}
