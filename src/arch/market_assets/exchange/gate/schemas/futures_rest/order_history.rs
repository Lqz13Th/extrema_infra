use serde::Deserialize;
use serde_json::Value;

use crate::arch::market_assets::{
    api_data::account_data::HistoOrderData,
    api_general::{ts_to_micros, value_to_f64},
    base_data::{OrderSide, OrderStatus, OrderType, TimeInForce},
    exchange::gate::api_utils::gate_fut_inst_to_cli,
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestFuturesOrderHistoryGateFutures {
    pub id: i64,
    pub contract: String,
    pub size: Value,
    pub left: Value,
    pub price: Value,
    pub fill_price: Value,
    pub status: String,
    pub finish_as: Option<String>,
    pub create_time: f64,
    pub update_time: Option<f64>,
    pub finish_time: Option<f64>,
    pub text: Option<String>,
    pub tif: Option<String>,
}

impl From<RestFuturesOrderHistoryGateFutures> for HistoOrderData {
    fn from(d: RestFuturesOrderHistoryGateFutures) -> Self {
        let size = value_to_f64(&d.size);
        let left = value_to_f64(&d.left);
        let order_price = value_to_f64(&d.price);
        let fill_price = value_to_f64(&d.fill_price);
        let size_abs = size.abs();
        let left_abs = left.abs();

        HistoOrderData {
            timestamp: ts_to_micros(d.create_time as u64),
            inst: gate_fut_inst_to_cli(&d.contract),
            order_id: d.id.to_string(),
            cli_order_id: d.text.filter(|text| !text.is_empty() && text != "-"),
            side: if size >= 0.0 {
                OrderSide::BUY
            } else {
                OrderSide::SELL
            },
            position_side: None,
            order_type: parse_order_type(order_price, d.tif.as_deref()),
            order_status: parse_status(&d.status, d.finish_as.as_deref(), size_abs, left_abs),
            price: order_price,
            avg_price: if fill_price > 0.0 { fill_price } else { 0.0 },
            size: size_abs,
            executed_size: (size_abs - left_abs).max(0.0),
            fee: None,
            fee_currency: None,
            reduce_only: None,
            time_in_force: parse_time_in_force(d.tif.as_deref()),
            update_time: ts_to_micros(
                d.finish_time
                    .or(d.update_time)
                    .unwrap_or(d.create_time)
                    .max(d.create_time) as u64,
            ),
        }
    }
}

fn parse_status(
    status: &str,
    finish_as: Option<&str>,
    size_abs: f64,
    left_abs: f64,
) -> OrderStatus {
    match status {
        "open" => {
            if size_abs > left_abs {
                OrderStatus::PartiallyFilled
            } else {
                OrderStatus::Live
            }
        },
        "finished" => match finish_as {
            Some("filled") => OrderStatus::Filled,
            Some(
                "cancelled" | "liquidated" | "ioc" | "auto_deleveraged" | "reduce_only"
                | "position_closed" | "reduce_out" | "stp",
            ) => OrderStatus::Canceled,
            _ => {
                if left_abs == 0.0 {
                    OrderStatus::Filled
                } else {
                    OrderStatus::Canceled
                }
            },
        },
        _ => OrderStatus::Unknown,
    }
}

fn parse_order_type(price: f64, tif: Option<&str>) -> OrderType {
    if price == 0.0 {
        return OrderType::Market;
    }

    match tif.unwrap_or_default().to_ascii_lowercase().as_str() {
        "poc" => OrderType::PostOnly,
        "ioc" => OrderType::Ioc,
        "fok" => OrderType::Fok,
        _ => OrderType::Limit,
    }
}

fn parse_time_in_force(tif: Option<&str>) -> Option<TimeInForce> {
    match tif.unwrap_or_default().to_ascii_lowercase().as_str() {
        "ioc" => Some(TimeInForce::IOC),
        "fok" => Some(TimeInForce::FOK),
        "gtc" | "poc" => Some(TimeInForce::GTC),
        _ => None,
    }
}
