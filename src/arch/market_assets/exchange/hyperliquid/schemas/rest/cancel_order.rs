use serde::Deserialize;

use crate::{
    arch::market_assets::{
        api_data::account_data::OrderAckData, api_general::get_micros_timestamp,
        base_data::OrderStatus,
    },
    errors::{InfraError, InfraResult},
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestCancelAckHyperliquid {
    pub statuses: Vec<RestCancelStatusHyperliquid>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum RestCancelStatusHyperliquid {
    Status(String),
    Error { error: String },
}

impl RestCancelAckHyperliquid {
    pub fn into_cancel_ack(
        self,
        order_id: Option<String>,
        cli_order_id: Option<String>,
    ) -> InfraResult<OrderAckData> {
        let status = self
            .statuses
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError(
                "No Hyperliquid cancel ack status returned".into(),
            ))?;
        let (order_status, msg) = match status {
            RestCancelStatusHyperliquid::Status(status) if status == "success" => {
                (OrderStatus::Canceled, None)
            },
            RestCancelStatusHyperliquid::Status(status) => (OrderStatus::Rejected, Some(status)),
            RestCancelStatusHyperliquid::Error { error } => (OrderStatus::Rejected, Some(error)),
        };

        Ok(OrderAckData {
            timestamp: get_micros_timestamp(),
            order_status,
            order_id: order_id.unwrap_or_default(),
            cli_order_id,
            msg,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_success_and_error_statuses() {
        let success: RestCancelAckHyperliquid =
            serde_json::from_str(r#"{"statuses":["success"]}"#).unwrap();
        let success = success.into_cancel_ack(Some("42".into()), None).unwrap();
        assert_eq!(success.order_status, OrderStatus::Canceled);
        assert_eq!(success.order_id, "42");

        let error: RestCancelAckHyperliquid =
            serde_json::from_str(r#"{"statuses":[{"error":"Order was never placed"}]}"#).unwrap();
        let error = error.into_cancel_ack(None, Some("0x123".into())).unwrap();
        assert_eq!(error.order_status, OrderStatus::Rejected);
        assert_eq!(error.msg.as_deref(), Some("Order was never placed"));
    }
}
