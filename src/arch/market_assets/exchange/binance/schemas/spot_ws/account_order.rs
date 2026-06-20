use serde::Deserialize;

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
pub(crate) struct WsAccountOrderEnvelopeBinanceSpot {
    subscriptionId: u64,
    event: WsAccountOrderBinanceSpot,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsAccountOrderBinanceSpot {
    E: u64,    // Event time
    s: String, // Symbol
    c: String, // Client order id
    #[serde(default)]
    i: Option<u64>, // Order id
    S: String, // Side
    o: String, // Order type
    q: String, // Original quantity
    p: String, // Original price
    L: String, // Filled price
    z: String, // Cumulative filled quantity
    X: String, // Order status
}

impl IntoWsData for WsAccountOrderEnvelopeBinanceSpot {
    type Output = WsAccOrder;

    fn into_ws(self) -> WsAccOrder {
        let _subscription_id = self.subscriptionId;
        let event = self.event;

        WsAccOrder {
            timestamp: ts_to_micros(event.E),
            market: Market::BinanceSpot,
            inst: event.s,
            inst_type: InstrumentType::Spot,
            price: event.L.parse().unwrap_or_default(),
            size: event.q.parse::<f64>().unwrap_or_default().abs(),
            filled_size: event.z.parse::<f64>().unwrap_or_default().abs(),
            side: match event.S.as_str() {
                "BUY" => OrderSide::BUY,
                "SELL" => OrderSide::SELL,
                _ => OrderSide::Unknown,
            },
            status: match event.X.as_str() {
                "NEW" => OrderStatus::Live,
                "PARTIALLY_FILLED" => OrderStatus::PartiallyFilled,
                "FILLED" => OrderStatus::Filled,
                "CANCELED" | "PENDING_CANCEL" => OrderStatus::Canceled,
                "EXPIRED" | "EXPIRED_IN_MATCH" => OrderStatus::Expired,
                "REJECTED" => OrderStatus::Rejected,
                _ => OrderStatus::Unknown,
            },
            order_type: match event.o.as_str() {
                "MARKET" => OrderType::Market,
                "LIMIT" | "LIMIT_MAKER" => OrderType::Limit,
                _ => OrderType::Unknown,
            },
            order_id: event.i.map(|id| id.to_string()),
            cli_order_id: Some(event.c),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::arch::traits::conversion::IntoWsData;

    use super::*;

    #[test]
    fn into_ws_preserves_exchange_and_client_order_ids() {
        let raw: WsAccountOrderEnvelopeBinanceSpot = serde_json::from_value(json!({
            "subscriptionId": 1_u64,
            "event": {
                "E": 1781905826733_u64,
                "s": "REUSDT",
                "c": "spot-client-id",
                "i": 370702544401581041_u64,
                "S": "BUY",
                "o": "MARKET",
                "q": "16",
                "p": "0",
                "L": "0.87295",
                "z": "16",
                "X": "FILLED"
            }
        }))
        .unwrap();

        let ws = raw.into_ws();

        assert_eq!(ws.order_id.as_deref(), Some("370702544401581041"));
        assert_eq!(ws.cli_order_id.as_deref(), Some("spot-client-id"));
    }
}
