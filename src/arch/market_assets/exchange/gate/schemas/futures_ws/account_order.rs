use serde::Deserialize;
use serde_json::Value;

use crate::arch::{
    market_assets::{
        api_general::{ts_to_micros, value_to_f64},
        base_data::{InstrumentType, OrderSide, OrderStatus, OrderType},
        exchange::gate::api_utils::{gate_fut_inst_to_cli, value_to_order_id},
        market_core::Market,
    },
    strategy_base::handler::lob_events::WsAccOrder,
    traits::conversion::IntoWsData,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsAccountOrderGateFutures {
    #[serde(default)]
    id: Option<Value>,
    contract: String,
    size: Value,
    left: Value,
    price: Value,
    fill_price: Value,
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

        let status = parse_status(
            &self.status,
            self.finish_as.as_deref(),
            filled_size,
            left_abs,
        );
        let order_price = value_to_f64(&self.price);
        let fill_price = value_to_f64(&self.fill_price);
        let ws_price = if fill_price > 0.0 {
            fill_price
        } else {
            order_price
        };
        let order_type = parse_order_type(order_price, self.tif.as_deref());
        let order_id = value_to_order_id(self.id.as_ref());

        let timestamp = self
            .update_time
            .map(ts_to_micros)
            .or_else(|| self.create_time_ms.map(ts_to_micros))
            .unwrap_or_default();

        WsAccOrder {
            timestamp,
            market: Market::GateFutures,
            inst: gate_fut_inst_to_cli(&self.contract),
            inst_type: InstrumentType::Perpetual,
            price: ws_price,
            size: size_abs,
            filled_size,
            side,
            status,
            order_type,
            order_id,
            cli_order_id: self.text.as_ref().and_then(|t| {
                if t.is_empty() || t == "-" {
                    None
                } else {
                    Some(t.clone())
                }
            }),
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
            Some(
                "cancelled" | "liquidated" | "ioc" | "auto_deleveraging" | "reduce_only"
                | "position_close" | "stp" | "reduce_out",
            ) => OrderStatus::Canceled,
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::arch::traits::conversion::IntoWsData;

    use super::*;

    #[test]
    fn into_ws_preserves_exchange_and_client_order_ids() {
        let raw: WsAccountOrderGateFutures = serde_json::from_value(json!({
            "id": 123456789_u64,
            "contract": "GUN_USDT",
            "size": -435,
            "left": 0,
            "price": "0",
            "fill_price": "0.005857",
            "status": "finished",
            "finish_as": "filled",
            "update_time": 1781905826_u64,
            "create_time_ms": 1781905826733_u64,
            "tif": "ioc",
            "text": "gate-futures-client-id"
        }))
        .unwrap();

        let ws = raw.into_ws();

        assert_eq!(ws.order_id.as_deref(), Some("123456789"));
        assert_eq!(ws.cli_order_id.as_deref(), Some("gate-futures-client-id"));
    }

    #[test]
    fn into_ws_converts_decimal_contract_size() {
        let raw: WsAccountOrderGateFutures = serde_json::from_value(json!({
            "id": "281193504063418585",
            "contract": "LAB_USDT",
            "size": "0.1",
            "left": "0",
            "price": "0",
            "fill_price": "14.64758",
            "status": "finished",
            "finish_as": "filled",
            "update_time": 1782726621_u64,
            "create_time_ms": 1782726621776_u64,
            "tif": "ioc",
            "text": "api"
        }))
        .unwrap();

        let ws = raw.into_ws();

        assert_eq!(ws.inst, "LAB_USDT_PERP");
        assert_eq!(ws.size, 0.1);
        assert_eq!(ws.filled_size, 0.1);
        assert_eq!(ws.status, OrderStatus::Filled);
    }
}
