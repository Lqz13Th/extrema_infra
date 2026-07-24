use serde::Deserialize;
use serde_json::Value;

use crate::arch::market_assets::{
    api_data::account_data::OrderDetailData,
    api_general::{ts_to_micros, value_to_f64},
    base_data::{OrderSide, OrderStatus, OrderType, TimeInForce},
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestOpenOrderHyperliquid {
    pub coin: String,
    pub side: String,
    pub limitPx: Value,
    pub sz: Value,
    pub oid: u64,
    pub timestamp: u64,
    pub origSz: Value,
    #[serde(default)]
    pub cloid: Option<String>,
    #[serde(default)]
    pub orderType: String,
    #[serde(default)]
    pub reduceOnly: Option<bool>,
    #[serde(default)]
    pub tif: Option<String>,
}

impl RestOpenOrderHyperliquid {
    pub fn into_order_detail_data(self, inst: &str) -> OrderDetailData {
        let remaining_size = value_to_f64(&self.sz).abs();
        let original_size = value_to_f64(&self.origSz).abs();
        let executed_size = (original_size - remaining_size).max(0.0);

        OrderDetailData {
            timestamp: ts_to_micros(self.timestamp),
            inst: inst.to_string(),
            order_id: self.oid.to_string(),
            cli_order_id: self.cloid.filter(|id| !id.is_empty()),
            side: match self.side.as_str() {
                "B" => OrderSide::BUY,
                "A" => OrderSide::SELL,
                _ => OrderSide::Unknown,
            },
            position_side: None,
            order_type: match self.tif.as_deref() {
                Some("Alo") => OrderType::PostOnly,
                Some("Ioc") => OrderType::Ioc,
                _ if self.orderType.eq_ignore_ascii_case("limit") => OrderType::Limit,
                _ if self.orderType.to_ascii_lowercase().contains("market") => OrderType::Market,
                _ => OrderType::Unknown,
            },
            order_status: if executed_size > 0.0 {
                OrderStatus::PartiallyFilled
            } else {
                OrderStatus::Live
            },
            price: value_to_f64(&self.limitPx),
            avg_price: 0.0,
            size: original_size,
            executed_size,
            fee: None,
            fee_currency: None,
            reduce_only: self.reduceOnly,
            time_in_force: match self.tif.as_deref() {
                Some("Gtc") => Some(TimeInForce::GTC),
                Some("Ioc") => Some(TimeInForce::IOC),
                _ => None,
            },
            update_time: ts_to_micros(self.timestamp),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_frontend_open_order() {
        let order: RestOpenOrderHyperliquid = serde_json::from_value(serde_json::json!({
            "coin": "BTC",
            "side": "A",
            "limitPx": "29792.0",
            "sz": "4.0",
            "oid": 91490942,
            "timestamp": 1681247412573_u64,
            "origSz": "5.0",
            "orderType": "Limit",
            "reduceOnly": false
        }))
        .unwrap();

        let order = order.into_order_detail_data("BTC_USDC_PERP");

        assert_eq!(order.inst, "BTC_USDC_PERP");
        assert_eq!(order.order_id, "91490942");
        assert_eq!(order.side, OrderSide::SELL);
        assert_eq!(order.order_status, OrderStatus::PartiallyFilled);
        assert_eq!(order.size, 5.0);
        assert_eq!(order.executed_size, 1.0);
    }
}
