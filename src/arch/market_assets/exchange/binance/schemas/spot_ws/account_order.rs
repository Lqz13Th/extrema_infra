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
    S: String, // Side
    o: String, // Order type
    q: String, // Original quantity
    p: String, // Original price
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
            price: event.p.parse().unwrap_or_default(),
            size: event.q.parse().unwrap_or_default(),
            filled_size: event.z.parse().unwrap_or_default(),
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
            cli_order_id: Some(event.c),
        }
    }
}
