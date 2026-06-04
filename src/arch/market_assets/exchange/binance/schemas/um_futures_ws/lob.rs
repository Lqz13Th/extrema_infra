use serde::Deserialize;

use crate::arch::{
    market_assets::{
        api_general::ts_to_micros, exchange::binance::api_utils::binance_fut_inst_to_cli,
        market_core::Market,
    },
    strategy_base::handler::lob_events::{LobEventKind, LobLevel, LobLevelAction, LobSeq, WsLob},
    traits::conversion::IntoWsData,
};

#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
pub(crate) struct WsBookTickerBinanceUM(BinanceBookTicker);

#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
pub(crate) struct WsPartialDepthBinanceUM(BinanceDepthBook);

#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
pub(crate) struct WsDiffDepthBinanceUM(BinanceDepthBook);

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct BinanceBookTicker {
    u: u64,
    s: String,
    b: String,
    B: String,
    a: String,
    A: String,
    T: u64,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct BinanceDepthBook {
    T: u64,
    s: String,
    U: u64,
    u: u64,
    pu: Option<u64>,
    b: Vec<BinanceLobLevel>,
    a: Vec<BinanceLobLevel>,
}

#[derive(Clone, Debug, Deserialize)]
struct BinanceLobLevel(String, String);

impl BinanceBookTicker {
    fn into_ws_lob(self) -> WsLob {
        let update_id = self.u;

        WsLob {
            timestamp: ts_to_micros(self.T),
            market: Market::BinanceUmFutures,
            inst: binance_fut_inst_to_cli(&self.s),
            event: LobEventKind::Bbo,
            bids: vec![binance_bbo_level(&self.b, &self.B, update_id)],
            asks: vec![binance_bbo_level(&self.a, &self.A, update_id)],
            seq: Some(LobSeq {
                prev: None,
                first: Some(update_id),
                last: Some(update_id),
            }),
            checksum: None,
        }
    }
}

impl BinanceDepthBook {
    fn into_ws_lob(self, event: LobEventKind) -> WsLob {
        let is_empty_update = self.b.is_empty() && self.a.is_empty();
        let delete_on_zero = matches!(event, LobEventKind::Incremental);

        WsLob {
            timestamp: ts_to_micros(self.T),
            market: Market::BinanceUmFutures,
            inst: binance_fut_inst_to_cli(&self.s),
            event: if matches!(event, LobEventKind::Incremental) && is_empty_update {
                LobEventKind::Heartbeat
            } else {
                event
            },
            bids: self
                .b
                .into_iter()
                .map(|level| binance_lob_level(level, delete_on_zero))
                .collect(),
            asks: self
                .a
                .into_iter()
                .map(|level| binance_lob_level(level, delete_on_zero))
                .collect(),
            seq: Some(LobSeq {
                prev: self.pu,
                first: Some(self.U),
                last: Some(self.u),
            }),
            checksum: None,
        }
    }
}

fn binance_bbo_level(price: &str, size: &str, update_id: u64) -> LobLevel {
    let size = size.parse().unwrap_or_default();

    LobLevel {
        price: price.parse().unwrap_or_default(),
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

fn binance_lob_level(level: BinanceLobLevel, delete_on_zero: bool) -> LobLevel {
    let size = level.1.parse().unwrap_or_default();

    LobLevel {
        price: level.0.parse().unwrap_or_default(),
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

impl IntoWsData for WsBookTickerBinanceUM {
    type Output = WsLob;

    fn into_ws(self) -> Self::Output {
        self.0.into_ws_lob()
    }
}

impl IntoWsData for WsPartialDepthBinanceUM {
    type Output = WsLob;

    fn into_ws(self) -> Self::Output {
        self.0.into_ws_lob(LobEventKind::Snapshot)
    }
}

impl IntoWsData for WsDiffDepthBinanceUM {
    type Output = WsLob;

    fn into_ws(self) -> Self::Output {
        self.0.into_ws_lob(LobEventKind::Incremental)
    }
}

#[cfg(test)]
mod tests {
    use crate::arch::{
        market_assets::{exchange::binance::binance_ws_msg::BinanceWsData, market_core::Market},
        strategy_base::handler::lob_events::{LobEventKind, LobLevelAction},
        traits::conversion::IntoWsData,
    };

    use super::*;

    #[test]
    fn parses_binance_um_book_ticker_as_bbo() {
        let raw = r#"{
            "e":"bookTicker",
            "u":10708444522016,
            "s":"BTCUSDT",
            "b":"63405.40",
            "B":"4.629",
            "a":"63405.50",
            "A":"2.357",
            "T":1780563843114,
            "E":1780563843114
        }"#;

        let parsed: BinanceWsData<WsBookTickerBinanceUM> = serde_json::from_str(raw).unwrap();
        let lob = parsed.into_ws();

        assert_eq!(lob.len(), 1);
        assert!(matches!(lob[0].event, LobEventKind::Bbo));
        assert_eq!(lob[0].market, Market::BinanceUmFutures);
        assert_eq!(lob[0].inst, "BTC_USDT_PERP");
        assert_eq!(lob[0].bids[0].price, 63405.40);
        assert_eq!(lob[0].bids[0].level_update_id, Some(10708444522016));
        assert_eq!(lob[0].seq.as_ref().unwrap().last, Some(10708444522016));
    }

    #[test]
    fn parses_binance_um_diff_depth_zero_size_as_delete() {
        let raw = r#"{
            "e":"depthUpdate",
            "E":1780563845145,
            "T":1780563845143,
            "s":"BTCUSDT",
            "U":10708445053618,
            "u":10708445072904,
            "pu":10708445053562,
            "b":[["50746.30","0.000"],["50748.80","0.002"]],
            "a":[["63436.10","1.801"]]
        }"#;

        let parsed: BinanceWsData<WsDiffDepthBinanceUM> = serde_json::from_str(raw).unwrap();
        let lob = parsed.into_ws();

        assert_eq!(lob.len(), 1);
        assert!(matches!(lob[0].event, LobEventKind::Incremental));
        assert_eq!(lob[0].market, Market::BinanceUmFutures);
        assert!(matches!(lob[0].bids[0].action, LobLevelAction::Delete));
        assert!(matches!(lob[0].bids[1].action, LobLevelAction::Upsert));
        assert_eq!(lob[0].seq.as_ref().unwrap().prev, Some(10708445053562));
        assert_eq!(lob[0].seq.as_ref().unwrap().last, Some(10708445072904));
    }
}
