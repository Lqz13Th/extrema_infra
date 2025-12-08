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
        OrderAckData {
            timestamp: ts_to_micros(d.ts.parse().unwrap_or_default()),
            order_status: if d.sCode == "0" {
                OrderStatus::Live
            } else {
                OrderStatus::Rejected
            },
            order_id: d.ordId,
            cli_order_id: d.clOrdId,
        }
    }
}
