use serde::Deserialize;
use tracing::warn;

use crate::arch::market_assets::{
    api_data::account_data::OrderDetailData,
    api_general::ts_to_micros,
    base_data::{OrderSide, OrderStatus, OrderType, TimeInForce},
    exchange::binance::api_utils::binance_spot_inst_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestOrderHistoryBinanceSpot {
    pub symbol: String,
    pub orderId: u64,
    pub clientOrderId: Option<String>,
    pub price: String,
    pub origQty: String,
    pub executedQty: String,
    pub cummulativeQuoteQty: Option<String>,
    pub status: String,
    pub timeInForce: String,
    pub r#type: String,
    pub side: String,
    pub time: u64,
    pub updateTime: u64,
}

impl From<RestOrderHistoryBinanceSpot> for OrderDetailData {
    fn from(d: RestOrderHistoryBinanceSpot) -> Self {
        let executed_size = d.executedQty.parse::<f64>().unwrap_or_default().abs();
        let cumulative_quote = d
            .cummulativeQuoteQty
            .as_deref()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or_default()
            .abs();

        OrderDetailData {
            timestamp: ts_to_micros(d.time),
            inst: binance_spot_inst_to_cli(&d.symbol),
            order_id: d.orderId.to_string(),
            cli_order_id: d.clientOrderId.filter(|id| !id.is_empty()),
            side: match d.side.as_str() {
                "BUY" => OrderSide::BUY,
                "SELL" => OrderSide::SELL,
                other => {
                    warn!("Unknown Binance Spot order side: {other}");
                    OrderSide::Unknown
                },
            },
            position_side: None,
            order_type: match d.r#type.as_str() {
                "MARKET" => OrderType::Market,
                "LIMIT" => OrderType::Limit,
                "LIMIT_MAKER" => OrderType::PostOnly,
                other => {
                    warn!("Unknown Binance Spot order type: {other}");
                    OrderType::Unknown
                },
            },
            order_status: match d.status.as_str() {
                "PENDING_NEW" | "NEW" => OrderStatus::Live,
                "PARTIALLY_FILLED" => OrderStatus::PartiallyFilled,
                "FILLED" => OrderStatus::Filled,
                "PENDING_CANCEL" | "CANCELED" => OrderStatus::Canceled,
                "REJECTED" => OrderStatus::Rejected,
                "EXPIRED" | "EXPIRED_IN_MATCH" => OrderStatus::Expired,
                other => {
                    warn!("Unknown Binance Spot order status: {other}");
                    OrderStatus::Unknown
                },
            },
            price: d.price.parse().unwrap_or_default(),
            avg_price: if executed_size > 0.0 {
                cumulative_quote / executed_size
            } else {
                0.0
            },
            size: d.origQty.parse::<f64>().unwrap_or_default().abs(),
            executed_size,
            fee: None,
            fee_currency: None,
            reduce_only: None,
            time_in_force: match d.timeInForce.as_str() {
                "GTC" => Some(TimeInForce::GTC),
                "IOC" => Some(TimeInForce::IOC),
                "FOK" => Some(TimeInForce::FOK),
                "" => None,
                other => {
                    warn!("Unknown Binance Spot time in force: {other}");
                    Some(TimeInForce::Unknown)
                },
            },
            update_time: ts_to_micros(d.updateTime),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn converts_filled_market_order_with_computed_average_price() {
        let raw: RestOrderHistoryBinanceSpot = serde_json::from_value(json!({
            "symbol": "USDCUSDT",
            "orderId": 1198981242_u64,
            "clientOrderId": "bridge-order-1",
            "price": "0.00000000",
            "origQty": "7.00000000",
            "executedQty": "7.00000000",
            "cummulativeQuoteQty": "6.99930000",
            "status": "FILLED",
            "timeInForce": "GTC",
            "type": "MARKET",
            "side": "BUY",
            "time": 1783580000123_u64,
            "updateTime": 1783580000456_u64
        }))
        .unwrap();

        let order = OrderDetailData::from(raw);

        assert_eq!(order.inst, "USDC_USDT");
        assert_eq!(order.order_status, OrderStatus::Filled);
        assert_eq!(order.executed_size, 7.0);
        assert!((order.avg_price - 0.9999).abs() < 1e-12);
        assert_eq!(order.update_time, 1_783_580_000_456_000);
    }

    #[test]
    fn maps_partially_filled_post_only_order() {
        let raw: RestOrderHistoryBinanceSpot = serde_json::from_value(json!({
            "symbol": "BTCUSDT",
            "orderId": 42_u64,
            "clientOrderId": "",
            "price": "60000",
            "origQty": "0.01",
            "executedQty": "0.004",
            "cummulativeQuoteQty": "240",
            "status": "PARTIALLY_FILLED",
            "timeInForce": "GTC",
            "type": "LIMIT_MAKER",
            "side": "SELL",
            "time": 1783580000123_u64,
            "updateTime": 1783580000456_u64
        }))
        .unwrap();

        let order = OrderDetailData::from(raw);

        assert_eq!(order.order_type, OrderType::PostOnly);
        assert_eq!(order.order_status, OrderStatus::PartiallyFilled);
        assert_eq!(order.cli_order_id, None);
        assert_eq!(order.avg_price, 60_000.0);
    }
}
