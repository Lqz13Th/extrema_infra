use serde::Deserialize;
use serde_json::Value;

use crate::arch::market_assets::{
    api_data::account_data::HistoOrderData,
    api_general::{ts_to_micros, value_to_f64},
    base_data::{OrderSide, OrderStatus, OrderType},
    exchange::hyperliquid::api_utils::hyperliquid_inst_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestOrderStatusHyperliquid {
    pub order: RestBasicOrderHyperliquid,
    pub status: String,
    pub statusTimestamp: u64,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestBasicOrderHyperliquid {
    pub coin: String,
    pub side: String,
    pub limitPx: Value,
    pub sz: Value,
    pub oid: u64,
    pub timestamp: u64,
    pub origSz: Value,
    #[serde(default)]
    pub cloid: Option<String>,
}

impl From<RestOrderStatusHyperliquid> for HistoOrderData {
    fn from(d: RestOrderStatusHyperliquid) -> Self {
        let remaining_size = value_to_f64(&d.order.sz).abs();
        let orig_size = value_to_f64(&d.order.origSz).abs();
        let filled_size = (orig_size - remaining_size).max(0.0);

        HistoOrderData {
            timestamp: ts_to_micros(d.order.timestamp),
            inst: hyperliquid_inst_to_cli(&d.order.coin),
            order_id: d.order.oid.to_string(),
            cli_order_id: d.order.cloid.filter(|id| !id.is_empty()),
            side: match d.order.side.as_str() {
                "B" => OrderSide::BUY,
                "A" => OrderSide::SELL,
                _ => OrderSide::Unknown,
            },
            position_side: None,
            order_type: OrderType::Unknown,
            order_status: parse_order_status(&d.status, filled_size),
            price: value_to_f64(&d.order.limitPx),
            avg_price: 0.0,
            size: orig_size,
            executed_size: filled_size,
            fee: None,
            fee_currency: None,
            reduce_only: None,
            time_in_force: None,
            update_time: ts_to_micros(d.statusTimestamp.max(d.order.timestamp)),
        }
    }
}

fn parse_order_status(status: &str, filled_size: f64) -> OrderStatus {
    let status = status.to_ascii_lowercase();

    if status == "open" || status == "triggered" {
        if filled_size > 0.0 {
            OrderStatus::PartiallyFilled
        } else {
            OrderStatus::Live
        }
    } else if status == "filled" {
        OrderStatus::Filled
    } else if status.contains("cancel") {
        OrderStatus::Canceled
    } else if status.contains("reject") {
        OrderStatus::Rejected
    } else {
        OrderStatus::Unknown
    }
}
