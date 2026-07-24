use serde::Deserialize;
use tracing::warn;

use crate::arch::market_assets::{
    api_data::account_data::OrderDetailData,
    api_general::ts_to_micros,
    base_data::{OrderSide, OrderStatus, OrderType, TimeInForce},
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestOrderHistoryGateSpot {
    pub id: String,
    pub text: Option<String>,
    pub create_time: Option<String>,
    pub update_time: Option<String>,
    #[serde(default)]
    pub create_time_ms: u64,
    #[serde(default)]
    pub update_time_ms: u64,
    pub currency_pair: String,
    pub status: String,
    pub r#type: String,
    pub side: String,
    pub amount: String,
    pub price: String,
    pub time_in_force: Option<String>,
    pub left: String,
    pub filled_amount: Option<String>,
    pub filled_total: Option<String>,
    pub avg_deal_price: Option<String>,
    pub fee: Option<String>,
    pub fee_currency: Option<String>,
    pub finish_as: Option<String>,
}

impl From<RestOrderHistoryGateSpot> for OrderDetailData {
    fn from(d: RestOrderHistoryGateSpot) -> Self {
        let amount = d.amount.parse::<f64>().unwrap_or_default().abs();
        let left = d.left.parse::<f64>().unwrap_or_default().abs();
        let filled_total = d
            .filled_total
            .as_deref()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or_default()
            .abs();
        let reported_filled = d
            .filled_amount
            .as_deref()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or_default()
            .abs();
        let reported_avg_price = d
            .avg_deal_price
            .as_deref()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or_default()
            .abs();
        let avg_price = if reported_avg_price > 0.0 {
            reported_avg_price
        } else if reported_filled > 0.0 {
            filled_total / reported_filled
        } else {
            0.0
        };
        let is_market_buy = d.r#type == "market" && d.side == "buy";
        let executed_size = if reported_filled > 0.0 {
            reported_filled
        } else if is_market_buy && avg_price > 0.0 {
            filled_total / avg_price
        } else if !is_market_buy {
            (amount - left).max(0.0)
        } else {
            0.0
        };
        let timestamp = if d.create_time_ms > 0 {
            d.create_time_ms
        } else {
            d.create_time
                .as_deref()
                .and_then(|value| value.parse::<f64>().ok())
                .map(|value| value as u64)
                .unwrap_or_default()
        };
        let update_time = if d.update_time_ms > 0 {
            d.update_time_ms
        } else {
            d.update_time
                .as_deref()
                .and_then(|value| value.parse::<f64>().ok())
                .map(|value| value as u64)
                .unwrap_or_default()
        };

        OrderDetailData {
            timestamp: ts_to_micros(timestamp),
            inst: d.currency_pair,
            order_id: d.id,
            cli_order_id: d.text.filter(|text| !text.is_empty() && text != "-"),
            side: match d.side.as_str() {
                "buy" => OrderSide::BUY,
                "sell" => OrderSide::SELL,
                other => {
                    warn!("Unknown Gate Spot order side: {other}");
                    OrderSide::Unknown
                },
            },
            position_side: None,
            order_type: if d.r#type == "market" {
                OrderType::Market
            } else {
                match d.time_in_force.as_deref().unwrap_or_default() {
                    "poc" => OrderType::PostOnly,
                    "ioc" => OrderType::Ioc,
                    "fok" => OrderType::Fok,
                    _ if d.r#type == "limit" => OrderType::Limit,
                    _ => {
                        warn!("Unknown Gate Spot order type: {}", d.r#type);
                        OrderType::Unknown
                    },
                }
            },
            order_status: match d.status.as_str() {
                "open" => {
                    if executed_size > 0.0 {
                        OrderStatus::PartiallyFilled
                    } else {
                        OrderStatus::Live
                    }
                },
                "closed" | "cancelled" => match d.finish_as.as_deref() {
                    Some("filled") => OrderStatus::Filled,
                    Some(
                        "cancelled"
                        | "liquidate_cancelled"
                        | "small"
                        | "depth_not_enough"
                        | "trader_not_enough"
                        | "ioc"
                        | "poc"
                        | "fok"
                        | "stp",
                    ) => OrderStatus::Canceled,
                    _ if left == 0.0 && executed_size > 0.0 => OrderStatus::Filled,
                    _ => OrderStatus::Canceled,
                },
                other => {
                    warn!("Unknown Gate Spot order status: {other}");
                    OrderStatus::Unknown
                },
            },
            price: d.price.parse::<f64>().unwrap_or_default().abs(),
            avg_price,
            size: if is_market_buy { executed_size } else { amount },
            executed_size,
            fee: d.fee.and_then(|value| value.parse::<f64>().ok()),
            fee_currency: d.fee_currency.filter(|currency| !currency.is_empty()),
            reduce_only: None,
            time_in_force: match d.time_in_force.as_deref().unwrap_or_default() {
                "gtc" | "poc" => Some(TimeInForce::GTC),
                "ioc" => Some(TimeInForce::IOC),
                "fok" => Some(TimeInForce::FOK),
                "" => None,
                other => {
                    warn!("Unknown Gate Spot time in force: {other}");
                    Some(TimeInForce::Unknown)
                },
            },
            update_time: ts_to_micros(update_time),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn converts_filled_market_buy_to_base_executed_size() {
        let raw: RestOrderHistoryGateSpot = serde_json::from_value(json!({
            "id": "241787005244120542",
            "text": "t-bridge-order-1",
            "create_time": "1783580000",
            "update_time": "1783580001",
            "create_time_ms": 1783580000123_u64,
            "update_time_ms": 1783580001456_u64,
            "currency_pair": "USDC_USDT",
            "status": "closed",
            "type": "market",
            "side": "buy",
            "amount": "7.0",
            "price": "0",
            "time_in_force": "ioc",
            "left": "0",
            "filled_amount": "7.00105016",
            "filled_total": "7.0",
            "avg_deal_price": "0.99985",
            "fee": "0.00700105",
            "fee_currency": "USDC",
            "finish_as": "filled"
        }))
        .unwrap();

        let order = OrderDetailData::from(raw);

        assert_eq!(order.order_status, OrderStatus::Filled);
        assert_eq!(order.order_type, OrderType::Market);
        assert_eq!(order.executed_size, 7.00105016);
        assert_eq!(order.size, order.executed_size);
        assert_eq!(order.avg_price, 0.99985);
        assert_eq!(order.fee_currency.as_deref(), Some("USDC"));
    }

    #[test]
    fn preserves_partial_fill_when_ioc_remainder_is_cancelled() {
        let raw: RestOrderHistoryGateSpot = serde_json::from_value(json!({
            "id": "43",
            "text": "-",
            "create_time": "1783580000",
            "update_time": "1783580001",
            "currency_pair": "LA_USDT",
            "status": "cancelled",
            "type": "limit",
            "side": "sell",
            "amount": "19",
            "price": "0.0579",
            "time_in_force": "ioc",
            "left": "3",
            "filled_amount": "16",
            "filled_total": "0.9264",
            "avg_deal_price": "0.0579",
            "fee": "0.001",
            "fee_currency": "USDT",
            "finish_as": "ioc"
        }))
        .unwrap();

        let order = OrderDetailData::from(raw);

        assert_eq!(order.order_status, OrderStatus::Canceled);
        assert_eq!(order.order_type, OrderType::Ioc);
        assert_eq!(order.executed_size, 16.0);
        assert_eq!(order.cli_order_id, None);
        assert_eq!(order.timestamp, 1_783_580_000_000_000);
    }
}
