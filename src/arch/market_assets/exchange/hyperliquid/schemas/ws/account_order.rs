use serde::Deserialize;

use crate::arch::{
    market_assets::{
        api_general::ts_to_micros,
        base_data::{InstrumentType, OrderSide, OrderStatus, OrderType},
        exchange::hyperliquid::api_utils::hyperliquid_inst_to_cli,
        market_core::Market,
    },
    strategy_base::handler::lob_events::WsAccOrder,
    traits::conversion::IntoWsData,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsAccountOrderHyperliquid {
    order: WsBasicOrderHyperliquid,
    status: String,
    statusTimestamp: u64,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct WsBasicOrderHyperliquid {
    coin: String,
    side: String,
    limitPx: String,
    sz: String,
    oid: u64,
    timestamp: u64,
    origSz: String,
    #[serde(default)]
    cloid: Option<String>,
}

impl IntoWsData for WsAccountOrderHyperliquid {
    type Output = WsAccOrder;

    fn into_ws(self) -> Self::Output {
        let remaining_size = self.order.sz.parse::<f64>().unwrap_or_default().abs();
        let orig_size = self.order.origSz.parse::<f64>().unwrap_or_default().abs();
        let filled_size = (orig_size - remaining_size).max(0.0);

        WsAccOrder {
            timestamp: ts_to_micros(self.statusTimestamp.max(self.order.timestamp)),
            market: Market::HyperLiquid,
            inst: hyperliquid_inst_to_cli(&self.order.coin),
            inst_type: infer_inst_type(&self.order.coin),
            price: self.order.limitPx.parse().unwrap_or_default(),
            size: orig_size,
            filled_size,
            side: match self.order.side.as_str() {
                "B" => OrderSide::BUY,
                "A" => OrderSide::SELL,
                _ => OrderSide::Unknown,
            },
            status: parse_order_status(&self.status, filled_size),
            order_type: OrderType::Limit,
            cli_order_id: self.order.cloid.filter(|cloid| !cloid.is_empty()),
        }
    }
}

fn infer_inst_type(coin: &str) -> InstrumentType {
    if coin.contains('/') || coin.starts_with('@') {
        InstrumentType::Spot
    } else {
        InstrumentType::Perpetual
    }
}

fn parse_order_status(status: &str, filled_size: f64) -> OrderStatus {
    let status = status.to_ascii_lowercase();

    if status == "open" || status == "triggered" {
        if filled_size > 0.0 {
            OrderStatus::PartiallyFilled
        } else {
            OrderStatus::Live
        }
    } else if status == "filled" {
        OrderStatus::Filled
    } else if status.contains("rejected") {
        OrderStatus::Rejected
    } else if status.contains("canceled") || status.contains("cancelled") {
        OrderStatus::Canceled
    } else {
        OrderStatus::Unknown
    }
}
