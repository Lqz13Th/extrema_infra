use serde::Deserialize;
use serde_json::Value;

use crate::arch::{
    market_assets::{
        api_general::{ts_to_micros, value_to_f64},
        exchange::gate::api_utils::gate_fut_inst_to_cli,
        market_core::Market,
    },
    strategy_base::handler::lob_events::{LobEventKind, LobLevel, LobLevelAction, LobSeq, WsLob},
    traits::conversion::IntoWsData,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsBookTickerGateFutures {
    t: u64,
    u: u64,
    s: String,
    b: Value,
    B: Value,
    a: Value,
    A: Value,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsOrderBookGateFutures {
    t: u64,
    id: u64,
    contract: String,
    asks: Vec<GateLobLevel>,
    bids: Vec<GateLobLevel>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsOrderBookUpdateGateFutures {
    t: u64,
    U: u64,
    u: u64,
    s: String,
    a: Vec<GateLobLevel>,
    b: Vec<GateLobLevel>,
}

#[derive(Clone, Debug, Deserialize)]
struct GateLobLevel {
    p: Value,
    s: Value,
}

impl WsBookTickerGateFutures {
    fn into_ws_lob(self) -> WsLob {
        WsLob {
            timestamp: ts_to_micros(self.t),
            market: Market::GateFutures,
            inst: gate_fut_inst_to_cli(&self.s),
            event: LobEventKind::Bbo,
            bids: vec![gate_bbo_level(self.b, self.B, self.u)],
            asks: vec![gate_bbo_level(self.a, self.A, self.u)],
            seq: Some(LobSeq {
                prev: None,
                first: Some(self.u),
                last: Some(self.u),
            }),
            checksum: None,
        }
    }
}

impl WsOrderBookGateFutures {
    fn into_ws_lob(self) -> WsLob {
        WsLob {
            timestamp: ts_to_micros(self.t),
            market: Market::GateFutures,
            inst: gate_fut_inst_to_cli(&self.contract),
            event: LobEventKind::Snapshot,
            bids: self
                .bids
                .into_iter()
                .map(|level| gate_lob_level(level, false))
                .collect(),
            asks: self
                .asks
                .into_iter()
                .map(|level| gate_lob_level(level, false))
                .collect(),
            seq: Some(LobSeq {
                prev: None,
                first: Some(self.id),
                last: Some(self.id),
            }),
            checksum: None,
        }
    }
}

impl WsOrderBookUpdateGateFutures {
    fn into_ws_lob(self) -> WsLob {
        let is_empty_update = self.a.is_empty() && self.b.is_empty();

        WsLob {
            timestamp: ts_to_micros(self.t),
            market: Market::GateFutures,
            inst: gate_fut_inst_to_cli(&self.s),
            event: if is_empty_update {
                LobEventKind::Heartbeat
            } else {
                LobEventKind::Incremental
            },
            bids: self
                .b
                .into_iter()
                .map(|level| gate_lob_level(level, true))
                .collect(),
            asks: self
                .a
                .into_iter()
                .map(|level| gate_lob_level(level, true))
                .collect(),
            seq: Some(LobSeq {
                prev: None,
                first: Some(self.U),
                last: Some(self.u),
            }),
            checksum: None,
        }
    }
}

fn gate_bbo_level(price: Value, size: Value, update_id: u64) -> LobLevel {
    let size = value_to_f64(&size);

    LobLevel {
        price: value_to_f64(&price),
        size,
        action: if size == 0.0 {
            LobLevelAction::Delete
        } else {
            LobLevelAction::Upsert
        },
        order_count: None,
        level_update_id: Some(update_id),
    }
}

fn gate_lob_level(level: GateLobLevel, delete_on_zero: bool) -> LobLevel {
    let size = value_to_f64(&level.s);

    LobLevel {
        price: value_to_f64(&level.p),
        size,
        action: if delete_on_zero && size == 0.0 {
            LobLevelAction::Delete
        } else {
            LobLevelAction::Upsert
        },
        order_count: None,
        level_update_id: None,
    }
}

impl IntoWsData for WsBookTickerGateFutures {
    type Output = WsLob;

    fn into_ws(self) -> WsLob {
        self.into_ws_lob()
    }
}

impl IntoWsData for WsOrderBookGateFutures {
    type Output = WsLob;

    fn into_ws(self) -> WsLob {
        self.into_ws_lob()
    }
}

impl IntoWsData for WsOrderBookUpdateGateFutures {
    type Output = WsLob;

    fn into_ws(self) -> WsLob {
        self.into_ws_lob()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::arch::{
        market_assets::exchange::gate::{
            gate_ws_msg::GateWsData,
            schemas::futures_ws::lob::{
                WsBookTickerGateFutures, WsOrderBookGateFutures, WsOrderBookUpdateGateFutures,
            },
        },
        strategy_base::handler::lob_events::{LobEventKind, LobLevelAction, WsLob},
        traits::conversion::IntoWsData,
    };

    #[test]
    fn parses_gate_book_ticker_as_bbo() {
        let raw = json!({
            "channel": "futures.book_ticker",
            "event": "update",
            "result": {
                "t": 1780569310140_u64,
                "u": 114427040898_u64,
                "s": "BTC_USDT",
                "b": "62706.4",
                "B": 3335,
                "a": "62706.5",
                "A": 8275
            }
        });

        let data: GateWsData<WsBookTickerGateFutures> = serde_json::from_value(raw).unwrap();
        let lob: Vec<WsLob> = data.into_ws();

        assert_eq!(lob.len(), 1);
        assert!(matches!(lob[0].event, LobEventKind::Bbo));
        assert_eq!(lob[0].inst, "BTC_USDT_PERP");
        assert_eq!(lob[0].timestamp, 1_780_569_310_140_000);
        assert_eq!(lob[0].bids[0].price, 62_706.4);
        assert_eq!(lob[0].bids[0].size, 3335.0);
        assert_eq!(lob[0].bids[0].level_update_id, Some(114427040898));
        assert_eq!(lob[0].seq.as_ref().unwrap().first, Some(114427040898));
    }

    #[test]
    fn parses_gate_order_book_snapshot() {
        let raw = json!({
            "channel": "futures.order_book",
            "event": "all",
            "result": {
                "t": 1780569310995_u64,
                "id": 114427042721_u64,
                "contract": "BTC_USDT",
                "asks": [{"p": "62698.1", "s": 10046}],
                "bids": [{"p": "62698", "s": 144730}],
                "l": "20"
            }
        });

        let data: GateWsData<WsOrderBookGateFutures> = serde_json::from_value(raw).unwrap();
        let lob: Vec<WsLob> = data.into_ws();

        assert_eq!(lob.len(), 1);
        assert!(matches!(lob[0].event, LobEventKind::Snapshot));
        assert_eq!(lob[0].seq.as_ref().unwrap().last, Some(114427042721));
        assert!(matches!(lob[0].asks[0].action, LobLevelAction::Upsert));
    }

    #[test]
    fn parses_gate_order_book_incremental_delete() {
        let raw = json!({
            "channel": "futures.order_book_update",
            "event": "update",
            "result": {
                "t": 1780569312795_u64,
                "U": 114427044935_u64,
                "u": 114427045214_u64,
                "s": "BTC_USDT",
                "a": [{"p": "62706.5", "s": 0}],
                "b": [],
                "l": "20"
            }
        });

        let data: GateWsData<WsOrderBookUpdateGateFutures> = serde_json::from_value(raw).unwrap();
        let lob: Vec<WsLob> = data.into_ws();

        assert_eq!(lob.len(), 1);
        assert!(matches!(lob[0].event, LobEventKind::Incremental));
        assert_eq!(lob[0].seq.as_ref().unwrap().first, Some(114427044935));
        assert_eq!(lob[0].seq.as_ref().unwrap().last, Some(114427045214));
        assert!(matches!(lob[0].asks[0].action, LobLevelAction::Delete));
    }
}
