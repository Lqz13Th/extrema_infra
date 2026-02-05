use serde::Deserialize;
use serde_json::Value;

use crate::arch::{
    market_assets::{
        api_general::{ts_to_micros, value_to_f64},
        base_data::{InstrumentType, OrderSide, OrderStatus, OrderType},
        exchange::gate::api_utils::gate_inst_to_cli,
        market_core::Market,
    },
    strategy_base::handler::lob_events::WsAccOrder,
    traits::conversion::IntoWsData,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsAccountOrderGateFutures {
    contract: String,
    size: Value,
    left: Value,
    price: Value,
    status: String,
    finish_as: Option<String>,
    update_time: Option<u64>,
    create_time_ms: Option<u64>,
    tif: Option<String>,
    text: Option<String>,
}

impl IntoWsData for WsAccountOrderGateFutures {
    type Output = WsAccOrder;

    fn into_ws(self) -> WsAccOrder {
        let size_val = value_to_f64(&self.size);
        let left_val = value_to_f64(&self.left);
        let size_abs = size_val.abs();
        let left_abs = left_val.abs();
        let filled_size = (size_abs - left_abs).max(0.0);

        let side = if size_val >= 0.0 {
            OrderSide::BUY
        } else {
            OrderSide::SELL
        };

        let status = parse_status(&self.status, self.finish_as.as_deref(), filled_size, left_abs);
        let order_type = parse_order_type(value_to_f64(&self.price), self.tif.as_deref());

        let timestamp = self
            .update_time
            .map(ts_to_micros)
            .or_else(|| self.create_time_ms.map(ts_to_micros))
            .unwrap_or_default();

        WsAccOrder {
            timestamp,
            market: Market::GateFutures,
            inst: gate_inst_to_cli(&self.contract),
            inst_type: InstrumentType::Perpetual,
            price: value_to_f64(&self.price),
            size: size_abs,
            filled_size,
            side,
            status,
            order_type,
            cli_order_id: self
                .text
                .as_ref()
                .and_then(|t| if t.is_empty() || t == "-" { None } else { Some(t.clone()) }),
        }
    }
}

fn parse_status(
    status: &str,
    finish_as: Option<&str>,
    filled_size: f64,
    left_abs: f64,
) -> OrderStatus {
    match status {
        "open" => {
            if filled_size > 0.0 {
                OrderStatus::PartiallyFilled
            } else {
                OrderStatus::Live
            }
        },
        "finished" => match finish_as {
            Some("filled") => OrderStatus::Filled,
            Some("cancelled" | "liquidated" | "ioc" | "auto_deleveraging" | "reduce_only"
            | "position_close" | "stp" | "reduce_out") => OrderStatus::Canceled,
            Some("_new") | Some("_update") => {
                if left_abs == 0.0 {
                    OrderStatus::Filled
                } else {
                    OrderStatus::Live
                }
            },
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

    match tif.unwrap_or_default().to_lowercase().as_str() {
        "poc" => OrderType::PostOnly,
        "ioc" => OrderType::Ioc,
        "fok" => OrderType::Fok,
        _ => OrderType::Limit,
    }
}
