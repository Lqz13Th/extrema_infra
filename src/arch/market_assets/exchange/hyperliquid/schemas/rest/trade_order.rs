use serde::Deserialize;

use crate::arch::market_assets::{api_data::account_data::OrderAckData, base_data::OrderStatus};

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
        match d.statuses.into_iter().next() {
            Some(RestOrderStatusHyperliquid::Resting { resting }) => OrderAckData {
                timestamp: 0,
                order_status: OrderStatus::Live,
                order_id: resting.oid.to_string(),
                cli_order_id: None,
                msg: None,
            },
            Some(RestOrderStatusHyperliquid::Filled { filled }) => OrderAckData {
                timestamp: 0,
                order_status: OrderStatus::Filled,
                order_id: filled.oid.to_string(),
                cli_order_id: None,
                msg: None,
            },
            Some(RestOrderStatusHyperliquid::Error { error }) => OrderAckData {
                timestamp: 0,
                order_status: OrderStatus::Rejected,
                order_id: String::new(),
                cli_order_id: None,
                msg: Some(error),
            },
            None => OrderAckData {
                timestamp: 0,
                order_status: OrderStatus::Unknown,
                order_id: String::new(),
                cli_order_id: None,
                msg: None,
            },
        }
    }
}
