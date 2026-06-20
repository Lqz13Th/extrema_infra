use serde::Deserialize;

use crate::arch::{
    market_assets::{
        api_general::ts_to_micros,
        base_data::{InstrumentType, OrderSide, OrderStatus, OrderType},
        exchange::binance::api_utils::binance_fut_inst_to_cli,
        market_core::Market,
    },
    strategy_base::handler::lob_events::WsAccOrder,
    traits::conversion::IntoWsData,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsAccountOrderBinanceUM {
    e: String, // Event type
    E: u64,    // Event time (ms)
    T: u64,    // Transaction time (ms)
    o: OrderUpdateDetail,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct OrderUpdateDetail {
    s: String,          // Symbol
    c: String,          // Client order id
    S: String,          // Side
    o: String,          // Order type
    f: String,          // Time in force
    q: String,          // Original quantity
    p: String,          // Original price
    ap: String,         // Average price
    sp: String,         // Stop price
    x: String,          // Execution type
    X: String,          // Order status
    i: u64,             // Order ID
    l: String,          // Last filled quantity
    z: String,          // Filled accumulated quantity
    L: String,          // Last filled price
    N: Option<String>,  // Commission asset
    n: Option<String>,  // Commission
    T: u64,             // Order trade time
    t: u64,             // Trade ID
    b: String,          // Bids notional
    a: String,          // Ask notional
    m: bool,            // Is maker?
    R: bool,            // Reduce only?
    wt: String,         // Working type
    ot: String,         // Original order type
    ps: String,         // Position side
    cp: bool,           // Close-all?
    AP: Option<String>, // Activation price
    cr: Option<String>, // Callback rate
    pP: bool,           // Price protection enabled?
    rp: String,         // Realized profit
    V: Option<String>,  // STP mode
    pm: Option<String>, // Price match mode
    gtd: Option<u64>,   // GTD auto cancel time
    er: Option<String>, // Expired reason
}

impl IntoWsData for WsAccountOrderBinanceUM {
    type Output = WsAccOrder;

    fn into_ws(self) -> WsAccOrder {
        WsAccOrder {
            timestamp: ts_to_micros(self.E),
            market: Market::BinanceUmFutures,
            inst: binance_fut_inst_to_cli(&self.o.s),
            inst_type: {
                if self.o.s.contains("_") {
                    InstrumentType::Futures
                } else {
                    InstrumentType::Perpetual
                }
            },
            price: self.o.ap.parse().unwrap_or_default(),
            size: self.o.q.parse().unwrap_or_default(),
            filled_size: self.o.z.parse().unwrap_or_default(),
            side: match self.o.S.as_str() {
                "BUY" => OrderSide::BUY,
                "SELL" => OrderSide::SELL,
                _ => OrderSide::Unknown,
            },
            status: match self.o.X.as_str() {
                "NEW" => OrderStatus::Live,
                "PARTIALLY_FILLED" => OrderStatus::PartiallyFilled,
                "FILLED" => OrderStatus::Filled,
                "CANCELED" => OrderStatus::Canceled,
                "EXPIRED" => OrderStatus::Expired,
                _ => OrderStatus::Unknown,
            },
            order_type: match self.o.o.as_str() {
                "MARKET" => OrderType::Market,
                "LIMIT" => OrderType::Limit,
                _ => OrderType::Unknown,
            },
            order_id: Some(self.o.i.to_string()),
            cli_order_id: Some(self.o.c),
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
        let raw: WsAccountOrderBinanceUM = serde_json::from_value(json!({
            "e": "ORDER_TRADE_UPDATE",
            "E": 1781905826733_u64,
            "T": 1781905826733_u64,
            "o": {
                "s": "GUNUSDT",
                "c": "CYA3pfUhF2yFO3kbBgIDMP",
                "S": "BUY",
                "o": "MARKET",
                "f": "GTC",
                "q": "4350",
                "p": "0",
                "ap": "0.005857",
                "sp": "0",
                "x": "TRADE",
                "X": "FILLED",
                "i": 1272696572_u64,
                "l": "4350",
                "z": "4350",
                "L": "0.005857",
                "N": "USDT",
                "n": "0",
                "T": 1781905826733_u64,
                "t": 1_u64,
                "b": "0",
                "a": "0",
                "m": false,
                "R": false,
                "wt": "CONTRACT_PRICE",
                "ot": "MARKET",
                "ps": "BOTH",
                "cp": false,
                "AP": null,
                "cr": null,
                "pP": false,
                "rp": "0",
                "V": "NONE",
                "pm": "NONE",
                "gtd": 0_u64,
                "er": "0"
            }
        }))
        .unwrap();

        let ws = raw.into_ws();

        assert_eq!(ws.order_id.as_deref(), Some("1272696572"));
        assert_eq!(ws.cli_order_id.as_deref(), Some("CYA3pfUhF2yFO3kbBgIDMP"));
    }
}
