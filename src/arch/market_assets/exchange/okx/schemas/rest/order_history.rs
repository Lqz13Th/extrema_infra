use serde::Deserialize;
use serde_json::Value;
use tracing::warn;

use crate::arch::market_assets::{
    api_data::account_data::HistoOrderData,
    api_general::ts_to_micros,
    base_data::{OrderSide, OrderStatus, OrderType, PositionSide, TimeInForce},
    exchange::okx::api_utils::okx_inst_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestOrderHistoryOkx {
    pub instId: String,
    pub ordId: String,
    pub clOrdId: Option<String>,
    pub side: String,
    pub posSide: Option<String>,
    pub ordType: String,
    pub state: String,
    pub px: Option<String>,
    pub sz: String,
    pub accFillSz: Option<String>,
    pub fillSz: Option<String>,
    pub avgPx: Option<String>,
    pub fee: Option<String>,
    pub feeCcy: Option<String>,
    pub reduceOnly: Option<Value>,
    pub cTime: Option<String>,
    pub uTime: Option<String>,
    pub fillTime: Option<String>,
}

impl From<RestOrderHistoryOkx> for HistoOrderData {
    fn from(d: RestOrderHistoryOkx) -> Self {
        let (order_type, time_in_force) = parse_order_kind(&d.ordType);
        let executed_size = d
            .accFillSz
            .as_deref()
            .or(d.fillSz.as_deref())
            .and_then(|sz| sz.parse::<f64>().ok())
            .unwrap_or_default()
            .abs();

        HistoOrderData {
            timestamp: ts_to_micros(parse_ts(d.cTime.as_deref())),
            inst: okx_inst_to_cli(&d.instId),
            order_id: d.ordId,
            cli_order_id: d.clOrdId.filter(|id| !id.is_empty()),
            side: match d.side.as_str() {
                "buy" => OrderSide::BUY,
                "sell" => OrderSide::SELL,
                other => {
                    warn!("Unknown OKX order side: {}", other);
                    OrderSide::Unknown
                },
            },
            position_side: d.posSide.as_deref().map(|side| match side {
                "long" => PositionSide::Long,
                "short" => PositionSide::Short,
                "net" => PositionSide::Both,
                other => {
                    warn!("Unknown OKX position side: {}", other);
                    PositionSide::Unknown
                },
            }),
            order_type,
            order_status: match d.state.as_str() {
                "live" => OrderStatus::Live,
                "partially_filled" => OrderStatus::PartiallyFilled,
                "filled" => OrderStatus::Filled,
                "canceled" | "mmp_canceled" => OrderStatus::Canceled,
                other => {
                    warn!("Unknown OKX order status: {}", other);
                    OrderStatus::Unknown
                },
            },
            price: d.px.and_then(|px| px.parse().ok()).unwrap_or_default(),
            avg_price: d.avgPx.and_then(|px| px.parse().ok()).unwrap_or_default(),
            size: d.sz.parse::<f64>().unwrap_or_default().abs(),
            executed_size,
            fee: d.fee.and_then(|fee| fee.parse().ok()),
            fee_currency: d.feeCcy.filter(|ccy| !ccy.is_empty()),
            reduce_only: parse_optional_bool(d.reduceOnly.as_ref()),
            time_in_force,
            update_time: ts_to_micros(
                parse_ts(d.uTime.as_deref()).max(parse_ts(d.fillTime.as_deref())),
            ),
        }
    }
}

fn parse_ts(raw: Option<&str>) -> u64 {
    raw.and_then(|ts| ts.parse::<u64>().ok())
        .unwrap_or_default()
}

fn parse_optional_bool(value: Option<&Value>) -> Option<bool> {
    value.and_then(|value| {
        value.as_bool().or_else(|| {
            value.as_str().and_then(|s| match s {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            })
        })
    })
}

fn parse_order_kind(ord_type: &str) -> (OrderType, Option<TimeInForce>) {
    match ord_type {
        "market" => (OrderType::Market, None),
        "limit" => (OrderType::Limit, Some(TimeInForce::GTC)),
        "post_only" => (OrderType::PostOnly, Some(TimeInForce::GTC)),
        "fok" => (OrderType::Fok, Some(TimeInForce::FOK)),
        "ioc" | "optimal_limit_ioc" => (OrderType::Ioc, Some(TimeInForce::IOC)),
        other => {
            warn!("Unknown OKX order type: {}", other);
            (OrderType::Unknown, None)
        },
    }
}
