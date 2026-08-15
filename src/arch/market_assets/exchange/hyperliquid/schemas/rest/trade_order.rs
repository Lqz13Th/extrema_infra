use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::account_data::OrderAckData, api_general::get_micros_timestamp, base_data::OrderStatus,
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestOrderAckHyperliquid {
    pub statuses: Vec<RestOrderStatusHyperliquid>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum RestOrderStatusHyperliquid {
    Resting {
        resting: RestOrderRestingHyperliquid,
    },
    Filled {
        filled: RestOrderFilledHyperliquid,
    },
    Error {
        error: String,
    },
}

#[derive(Clone, Debug, Deserialize)]
pub struct RestOrderRestingHyperliquid {
    pub oid: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RestOrderFilledHyperliquid {
    pub oid: u64,
}

impl From<RestOrderAckHyperliquid> for OrderAckData {
    fn from(d: RestOrderAckHyperliquid) -> Self {
        d.into_order_acks()
            .into_iter()
            .next()
            .unwrap_or_else(|| OrderAckData {
                timestamp: get_micros_timestamp(),
                order_status: OrderStatus::Unknown,
                order_id: String::new(),
                cli_order_id: None,
                msg: None,
            })
    }
}

impl RestOrderAckHyperliquid {
    pub fn into_order_acks(self) -> Vec<OrderAckData> {
        self.statuses
            .into_iter()
            .map(RestOrderStatusHyperliquid::into_order_ack)
            .collect()
    }
}

impl RestOrderStatusHyperliquid {
    fn into_order_ack(self) -> OrderAckData {
        match self {
            Self::Resting { resting } => OrderAckData {
                timestamp: get_micros_timestamp(),
                order_status: OrderStatus::Live,
                order_id: resting.oid.to_string(),
                cli_order_id: None,
                msg: None,
            },
            Self::Filled { filled } => OrderAckData {
                timestamp: get_micros_timestamp(),
                order_status: OrderStatus::Filled,
                order_id: filled.oid.to_string(),
                cli_order_id: None,
                msg: None,
            },
            Self::Error { error } => OrderAckData {
                timestamp: get_micros_timestamp(),
                order_status: OrderStatus::Rejected,
                order_id: String::new(),
                cli_order_id: None,
                msg: Some(error),
            },
        }
    }
}
