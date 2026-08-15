use serde::Deserialize;
use serde_json::Value;
use tracing::warn;

use crate::arch::market_assets::{
    api_data::account_data::OrderDetailData,
    api_general::ts_to_micros,
    base_data::{OrderSide, OrderStatus, OrderType, PositionSide, TimeInForce},
    exchange::okx::api_utils::okx_inst_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestAttachedAlgoOrderOkx {
    pub activePx: Option<String>,
    pub amendPxOnTriggerType: Option<String>,
    pub attachAlgoClOrdId: Option<String>,
    pub attachAlgoId: Option<String>,
    pub callbackRatio: Option<String>,
    pub callbackSpread: Option<String>,
    pub failCode: Option<String>,
    pub failReason: Option<String>,
    pub percent: Option<String>,
    pub slOrdPx: Option<String>,
    pub slTriggerPx: Option<String>,
    pub slTriggerPxType: Option<String>,
    pub slTriggerRatio: Option<String>,
    pub sz: Option<String>,
    pub tpOrdKind: Option<String>,
    pub tpOrdPx: Option<String>,
    pub tpTriggerPx: Option<String>,
    pub tpTriggerPxType: Option<String>,
    pub tpTriggerRatio: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestLinkedAlgoOrderOkx {
    pub algoId: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestOrderHistoryOkx {
    pub instType: Option<String>,
    pub instId: String,
    pub ordId: String,
    pub clOrdId: Option<String>,
    pub algoClOrdId: Option<String>,
    pub algoId: Option<String>,
    pub attachAlgoClOrdId: Option<String>,
    pub side: String,
    pub posSide: Option<String>,
    pub tdMode: Option<String>,
    pub ordType: String,
    pub state: String,
    pub category: Option<String>,
    pub source: Option<String>,
    pub isTpLimit: Option<String>,
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
    #[serde(default)]
    pub attachAlgoOrds: Vec<RestAttachedAlgoOrderOkx>,
    pub linkedAlgoOrd: Option<RestLinkedAlgoOrderOkx>,
    pub tpOrdPx: Option<String>,
    pub tpTriggerPx: Option<String>,
    pub tpTriggerPxType: Option<String>,
    pub slOrdPx: Option<String>,
    pub slTriggerPx: Option<String>,
    pub slTriggerPxType: Option<String>,
}

impl From<RestOrderHistoryOkx> for OrderDetailData {
    fn from(d: RestOrderHistoryOkx) -> Self {
        let (order_type, time_in_force) = parse_order_kind(&d.ordType);
        let executed_size = d
            .accFillSz
            .as_deref()
            .or(d.fillSz.as_deref())
            .and_then(|sz| sz.parse::<f64>().ok())
            .unwrap_or_default()
            .abs();

        OrderDetailData {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn order_fixture() -> serde_json::Value {
        serde_json::from_str(
            r#"{
                "instId": "BTC-USDT-SWAP",
                "instType": "SWAP",
                "ordId": "12345",
                "clOrdId": "entry-1",
                "algoClOrdId": "",
                "algoId": "",
                "attachAlgoClOrdId": "bracket-1",
                "side": "buy",
                "posSide": "long",
                "tdMode": "cross",
                "ordType": "limit",
                "state": "filled",
                "category": "normal",
                "source": "",
                "isTpLimit": "false",
                "px": "100",
                "sz": "3",
                "accFillSz": "3",
                "fillSz": "1",
                "avgPx": "100.1",
                "fee": "-0.01",
                "feeCcy": "USDT",
                "reduceOnly": "false",
                "cTime": "1720000000000",
                "uTime": "1720000001000",
                "fillTime": "1720000000900",
                "tpOrdPx": "103.1",
                "tpTriggerPx": "103",
                "tpTriggerPxType": "last",
                "slOrdPx": "-1",
                "slTriggerPx": "95",
                "slTriggerPxType": "mark",
                "linkedAlgoOrd": {"algoId": "linked-1"},
                "attachAlgoOrds": [
                    {
                        "activePx": "",
                        "amendPxOnTriggerType": "1",
                        "attachAlgoClOrdId": "tp-1",
                        "attachAlgoId": "algo-1",
                        "callbackRatio": "",
                        "callbackSpread": "",
                        "failCode": "",
                        "failReason": "",
                        "percent": "",
                        "slOrdPx": "-1",
                        "slTriggerPx": "95",
                        "slTriggerPxType": "mark",
                        "slTriggerRatio": "",
                        "sz": "1",
                        "tpOrdKind": "condition",
                        "tpOrdPx": "101.1",
                        "tpTriggerPx": "101",
                        "tpTriggerPxType": "last",
                        "tpTriggerRatio": ""
                    },
                    {
                        "attachAlgoClOrdId": "tp-2",
                        "attachAlgoId": "algo-2",
                        "sz": "2",
                        "tpOrdKind": "condition",
                        "tpOrdPx": "102.1",
                        "tpTriggerPx": "102",
                        "tpTriggerPxType": "mark"
                    }
                ]
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn deserializes_attached_and_top_level_tp_sl_fields() {
        let order: RestOrderHistoryOkx = serde_json::from_value(order_fixture()).unwrap();

        assert_eq!(order.attachAlgoOrds.len(), 2);
        assert_eq!(order.instType.as_deref(), Some("SWAP"));
        assert_eq!(order.category.as_deref(), Some("normal"));
        assert_eq!(order.source.as_deref(), Some(""));
        assert_eq!(order.isTpLimit.as_deref(), Some("false"));
        assert_eq!(order.tdMode.as_deref(), Some("cross"));
        assert_eq!(order.attachAlgoClOrdId.as_deref(), Some("bracket-1"));
        assert_eq!(
            order
                .linkedAlgoOrd
                .as_ref()
                .and_then(|linked| linked.algoId.as_deref()),
            Some("linked-1")
        );
        assert_eq!(order.attachAlgoOrds[0].tpTriggerPx.as_deref(), Some("101"));
        assert_eq!(
            order.attachAlgoOrds[0].slTriggerPxType.as_deref(),
            Some("mark")
        );
        assert_eq!(order.attachAlgoOrds[1].sz.as_deref(), Some("2"));
        assert_eq!(order.tpTriggerPx.as_deref(), Some("103"));
        assert_eq!(order.slTriggerPx.as_deref(), Some("95"));
    }

    #[test]
    fn defaults_missing_attached_orders_to_empty() {
        let mut fixture = order_fixture();
        fixture.as_object_mut().unwrap().remove("attachAlgoOrds");

        let order: RestOrderHistoryOkx = serde_json::from_value(fixture).unwrap();

        assert!(order.attachAlgoOrds.is_empty());
    }

    #[test]
    fn preserves_normalized_order_conversion() {
        let raw: RestOrderHistoryOkx = serde_json::from_value(order_fixture()).unwrap();
        let normalized = OrderDetailData::from(raw);

        assert_eq!(normalized.timestamp, 1_720_000_000_000_000);
        assert_eq!(normalized.inst, "BTC_USDT_PERP");
        assert_eq!(normalized.order_id, "12345");
        assert_eq!(normalized.cli_order_id.as_deref(), Some("entry-1"));
        assert_eq!(normalized.side, OrderSide::BUY);
        assert_eq!(normalized.position_side, Some(PositionSide::Long));
        assert_eq!(normalized.order_type, OrderType::Limit);
        assert_eq!(normalized.order_status, OrderStatus::Filled);
        assert_eq!(normalized.price, 100.0);
        assert_eq!(normalized.avg_price, 100.1);
        assert_eq!(normalized.size, 3.0);
        assert_eq!(normalized.executed_size, 3.0);
        assert_eq!(normalized.fee, Some(-0.01));
        assert_eq!(normalized.fee_currency.as_deref(), Some("USDT"));
        assert_eq!(normalized.reduce_only, Some(false));
        assert_eq!(normalized.time_in_force, Some(TimeInForce::GTC));
        assert_eq!(normalized.update_time, 1_720_000_001_000_000);
    }
}
