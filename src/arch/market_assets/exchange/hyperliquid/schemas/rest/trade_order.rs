use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::account_data::OrderAckData, api_general::get_micros_timestamp, base_data::OrderStatus,
};
use crate::errors::{InfraError, InfraResult};

#[derive(Clone, Debug, Deserialize)]
pub struct RestOrderResponse {
    #[serde(rename = "type")]
    pub resp_type: String,
    pub data: Option<RestOrderData>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RestOrderData {
    pub statuses: Vec<RestOrderStatus>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum RestOrderStatus {
    Resting { resting: RestingStatus },
    Filled { filled: FilledStatus },
    Error { error: String },
}

#[derive(Clone, Debug, Deserialize)]
pub struct RestingStatus {
    pub oid: u64,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct FilledStatus {
    pub oid: u64,
    #[serde(default)]
    pub totalSz: String,
    #[serde(default)]
    pub avgPx: String,
}

impl RestOrderResponse {
    pub fn into_order_ack(self, cli_order_id: Option<String>) -> InfraResult<OrderAckData> {
        let data = self.data.ok_or(InfraError::ApiCliError(
            "Missing Hyperliquid order response data".into(),
        ))?;

        let status = data
            .statuses
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError(
                "Missing Hyperliquid order status".into(),
            ))?;

        let timestamp = get_micros_timestamp();

        match status {
            RestOrderStatus::Resting { resting } => Ok(OrderAckData {
                timestamp,
                order_status: OrderStatus::Live,
                order_id: resting.oid.to_string(),
                cli_order_id,
            }),
            RestOrderStatus::Filled { filled } => Ok(OrderAckData {
                timestamp,
                order_status: OrderStatus::Filled,
                order_id: filled.oid.to_string(),
                cli_order_id,
            }),
            RestOrderStatus::Error { error } => Err(InfraError::ApiCliError(format!(
                "Hyperliquid order error: {}",
                error
            ))),
        }
    }
}
