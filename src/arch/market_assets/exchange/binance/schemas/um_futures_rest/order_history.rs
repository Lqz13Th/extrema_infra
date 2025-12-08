use serde::Deserialize;
use tracing::warn;

use crate::arch::market_assets::{
    api_data::account_data::HistoOrderData,
    api_general::ts_to_micros,
    base_data::{OrderSide, OrderStatus, OrderType, PositionSide, TimeInForce},
    exchange::binance::api_utils::binance_inst_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestOrderHistoryBinanceUM {
    pub symbol: String,
    pub orderId: u64,
    pub clientOrderId: Option<String>,
    pub price: String,
    pub avgPrice: Option<String>,
    pub origQty: String,
    pub executedQty: String,
    pub cumQuote: String,
    pub status: String,
    pub timeInForce: String,
    pub r#type: String,
    pub side: String,
    pub positionSide: Option<String>,
    pub reduceOnly: Option<bool>,
    pub time: u64,
    pub updateTime: u64,
}

impl From<RestOrderHistoryBinanceUM> for HistoOrderData {
    fn from(d: RestOrderHistoryBinanceUM) -> Self {
        let side = match d.side.as_str() {
            "BUY" => OrderSide::BUY,
            "SELL" => OrderSide::SELL,
            other => {
                warn!("Unknown Binance order side: {}", other);
                OrderSide::Unknown
            },
        };

        let order_type = match d.r#type.as_str() {
            "MARKET" => OrderType::Market,
            "LIMIT" => OrderType::Limit,
            "POST_ONLY" => OrderType::PostOnly,
            "FOK" => OrderType::Fok,
            "IOC" => OrderType::Ioc,
            other => {
                warn!("Unknown Binance order type: {}", other);
                OrderType::Unknown
            },
        };

        let order_status = match d.status.as_str() {
            "NEW" => OrderStatus::Live,
            "PARTIALLY_FILLED" => OrderStatus::PartiallyFilled,
            "FILLED" => OrderStatus::Filled,
            "EXPIRED" => OrderStatus::Expired,
            "CANCELED" => OrderStatus::Canceled,
            "REJECTED" => OrderStatus::Rejected,
            other => {
                warn!("Unknown Binance order status: {}", other);
                OrderStatus::Unknown
            },
        };

        let time_in_force = match d.timeInForce.as_str() {
            "GTC" => TimeInForce::GTC,
            "IOC" => TimeInForce::IOC,
            "FOK" => TimeInForce::FOK,
            other => {
                warn!("Unknown Binance time in force: {}", other);
                TimeInForce::Unknown
            },
        };

        let position_side = d.positionSide.as_deref().map(|s| match s {
            "LONG" => PositionSide::Long,
            "SHORT" => PositionSide::Short,
            "BOTH" => PositionSide::Both,
            other => {
                warn!("Unknown Binance position side: {}", other);
                PositionSide::Unknown
            },
        });

        HistoOrderData {
            timestamp: ts_to_micros(d.time),
            inst: binance_inst_to_cli(&d.symbol),
            order_id: d.orderId.to_string(),
            cli_order_id: d.clientOrderId,
            side,
            order_type,
            order_status,
            price: d.price.parse().unwrap_or_default(),
            avg_price: d.avgPrice.and_then(|p| p.parse().ok()).unwrap_or_default(),
            size: d.origQty.parse().unwrap_or_default(),
            executed_size: d.executedQty.parse().unwrap_or_default(),
            fee: None,
            reduce_only: d.reduceOnly,
            position_side,
            time_in_force: Some(time_in_force),
            update_time: ts_to_micros(d.updateTime),
            fee_currency: None,
        }
    }
}
