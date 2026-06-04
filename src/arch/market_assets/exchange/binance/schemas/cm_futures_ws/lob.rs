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
pub(crate) struct WsBookTickerBinanceCM(BinanceBookTicker);

#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
pub(crate) struct WsPartialDepthBinanceCM(BinanceDepthBook);

#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
pub(crate) struct WsDiffDepthBinanceCM(BinanceDepthBook);

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
            market: Market::BinanceCmFutures,
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
            market: Market::BinanceCmFutures,
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

impl IntoWsData for WsBookTickerBinanceCM {
    type Output = WsLob;

    fn into_ws(self) -> Self::Output {
        self.0.into_ws_lob()
    }
}

impl IntoWsData for WsPartialDepthBinanceCM {
    type Output = WsLob;

    fn into_ws(self) -> Self::Output {
        self.0.into_ws_lob(LobEventKind::Snapshot)
    }
}

impl IntoWsData for WsDiffDepthBinanceCM {
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
    fn parses_binance_cm_partial_depth_as_snapshot() {
        let raw = r#"{
            "e":"depthUpdate",
            "E":1780563999230,
            "T":1780563999229,
            "s":"BTCUSD_PERP",
            "ps":"BTCUSD",
            "U":1691890103301,
            "u":1691890108316,
            "pu":1691890103299,
            "b":[["63298.6","5563"]],
            "a":[["63298.7","333"]]
        }"#;

        let parsed: BinanceWsData<WsPartialDepthBinanceCM> = serde_json::from_str(raw).unwrap();
        let lob = parsed.into_ws();

        assert_eq!(lob.len(), 1);
        assert!(matches!(lob[0].event, LobEventKind::Snapshot));
        assert_eq!(lob[0].market, Market::BinanceCmFutures);
        assert_eq!(lob[0].inst, "BTC_USD_PERP");
        assert_eq!(lob[0].seq.as_ref().unwrap().prev, Some(1691890103299));
        assert_eq!(lob[0].seq.as_ref().unwrap().first, Some(1691890103301));
        assert!(matches!(lob[0].bids[0].action, LobLevelAction::Upsert));
    }
}
