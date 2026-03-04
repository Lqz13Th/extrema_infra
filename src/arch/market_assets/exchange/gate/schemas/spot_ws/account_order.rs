use serde::Deserialize;
use serde_json::Value;

use crate::arch::{
    market_assets::{
        api_general::ts_to_micros,
        base_data::{InstrumentType, OrderSide, OrderStatus, OrderType},
        market_core::Market,
    },
    strategy_base::handler::lob_events::WsAccOrder,
    traits::conversion::IntoWsData,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsAccountOrderGateSpot {
    currency_pair: String,
    side: String,
    r#type: String,
    amount: String,
    price: String,
    left: String,
    filled_amount: Option<String>,
    avg_deal_price: Option<String>,
    status: Option<String>,
    finish_as: Option<String>,
    event: Option<String>,
    update_time_ms: Option<Value>,
    create_time_ms: Option<Value>,
    text: Option<String>,
}

impl IntoWsData for WsAccountOrderGateSpot {
    type Output = WsAccOrder;

    fn into_ws(self) -> WsAccOrder {
        let size = self.amount.parse::<f64>().unwrap_or_default();
        let left = self.left.parse::<f64>().unwrap_or_default();
        let filled_from_left = (size - left).max(0.0);
        let filled = self
            .filled_amount
            .as_deref()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(filled_from_left);
        let order_price = self.price.parse::<f64>().unwrap_or_default();
        let avg_deal_price = self
            .avg_deal_price
            .as_deref()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or_default();
        let ws_price = if avg_deal_price > 0.0 {
            avg_deal_price
        } else {
            order_price
        };

        let status = parse_status(
            self.status.as_deref(),
            self.finish_as.as_deref(),
            self.event.as_deref(),
            filled,
            left,
        );

        let timestamp_ms = self
            .update_time_ms
            .as_ref()
            .and_then(value_to_u64_ms)
            .or_else(|| self.create_time_ms.as_ref().and_then(value_to_u64_ms))
            .unwrap_or_default();

        WsAccOrder {
            timestamp: ts_to_micros(timestamp_ms),
            market: Market::GateSpot,
            inst: self.currency_pair,
            inst_type: InstrumentType::Spot,
            price: ws_price,
            size,
            filled_size: filled,
            side: match self.side.as_str() {
                "buy" => OrderSide::BUY,
                "sell" => OrderSide::SELL,
                _ => OrderSide::Unknown,
            },
            status,
            order_type: if self.r#type.starts_with("market") {
                OrderType::Market
            } else {
                OrderType::Limit
            },
            cli_order_id: self.text.and_then(|t| {
                if t.is_empty() || t == "-" {
                    None
                } else {
                    Some(t)
                }
            }),
        }
    }
}

fn parse_status(
    status: Option<&str>,
    finish_as: Option<&str>,
    event: Option<&str>,
    filled: f64,
    left: f64,
) -> OrderStatus {
    if let Some(status) = status {
        return match status {
            "open" => {
                if filled > 0.0 {
                    OrderStatus::PartiallyFilled
                } else {
                    OrderStatus::Live
                }
            },
            "closed" | "cancelled" => match finish_as {
                Some("filled") => OrderStatus::Filled,
                Some("cancelled" | "ioc" | "stp" | "poc" | "fok") => OrderStatus::Canceled,
                _ => {
                    if left == 0.0 {
                        OrderStatus::Filled
                    } else if matches!(event, Some("finish")) {
                        OrderStatus::Canceled
                    } else {
                        OrderStatus::Unknown
                    }
                },
            },
            _ => OrderStatus::Unknown,
        };
    }

    // spot.orders_v2 in unified account may not include `status`.
    // Fallback to `finish_as` / `event` plus volume to infer state.
    match finish_as {
        Some("filled") => OrderStatus::Filled,
        Some("cancelled" | "ioc" | "stp" | "poc" | "fok") => OrderStatus::Canceled,
        Some("open") => {
            if filled > 0.0 {
                OrderStatus::PartiallyFilled
            } else {
                OrderStatus::Live
            }
        },
        _ => {
            if left == 0.0 && filled > 0.0 {
                OrderStatus::Filled
            } else if matches!(event, Some("finish")) {
                OrderStatus::Canceled
            } else if matches!(event, Some("put")) {
                if filled > 0.0 {
                    OrderStatus::PartiallyFilled
                } else {
                    OrderStatus::Live
                }
            } else {
                OrderStatus::Unknown
            }
        },
    }
}

fn value_to_u64_ms(v: &Value) -> Option<u64> {
    v.as_u64().or_else(|| {
        v.as_str()
            .and_then(|s| s.split('.').next())
            .and_then(|s| s.parse::<u64>().ok())
    })
}
