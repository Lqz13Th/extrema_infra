use serde::{Deserialize, Deserializer};

use crate::arch::{
    market_assets::{
        api_general::ts_to_micros,
        exchange::okx::{
            api_utils::okx_inst_to_cli,
            okx_ws_msg::{IntoOkxWsData, WsArg},
        },
        market_core::Market,
    },
    strategy_base::handler::lob_events::{LobEventKind, LobLevel, LobLevelAction, LobSeq, WsLob},
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct OkxWsLobBook {
    asks: Vec<OkxLobLevel>,
    bids: Vec<OkxLobLevel>,
    ts: String,
    checksum: Option<i64>,
    prevSeqId: Option<i64>,
    seqId: Option<i64>,
    instId: Option<String>,
}

#[derive(Clone, Debug)]
struct OkxLobLevel {
    price: String,
    size: String,
    order_count: String,
}

impl<'de> Deserialize<'de> for OkxLobLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let [price, size, _deprecated_feature, order_count] =
            <[String; 4]>::deserialize(deserializer)?;

        Ok(Self {
            price,
            size,
            order_count,
        })
    }
}

impl IntoOkxWsData for OkxWsLobBook {
    type Output = WsLob;

    fn into_ws_with_okx_context(self, arg: &WsArg, action: Option<&str>) -> Self::Output {
        let channel_name = arg.channel.as_deref();
        let event = okx_lob_event_kind(
            channel_name,
            action,
            self.bids.is_empty(),
            self.asks.is_empty(),
        );
        let arg_inst = arg.instId.as_deref();
        let inst = self.instId.as_deref().or(arg_inst).unwrap_or_default();

        WsLob {
            timestamp: ts_to_micros(self.ts.parse().unwrap_or_default()),
            market: Market::Okx,
            inst: okx_inst_to_cli(inst),
            event,
            bids: self.bids.into_iter().map(Into::into).collect(),
            asks: self.asks.into_iter().map(Into::into).collect(),
            seq: okx_lob_seq(self.prevSeqId, self.seqId),
            checksum: self.checksum.map(|checksum| checksum.to_string()),
        }
    }
}

impl From<OkxLobLevel> for LobLevel {
    fn from(level: OkxLobLevel) -> Self {
        let size = level.size.parse().unwrap_or_default();

        LobLevel {
            price: level.price.parse().unwrap_or_default(),
            size,
            // OKX incremental book updates delete a price level by sending size = 0.
            action: if size == 0.0 {
                LobLevelAction::Delete
            } else {
                LobLevelAction::Upsert
            },
            order_count: level.order_count.parse().ok(),
            level_update_id: None,
        }
    }
}

fn okx_lob_event_kind(
    channel_name: Option<&str>,
    action: Option<&str>,
    bids_empty: bool,
    asks_empty: bool,
) -> LobEventKind {
    match (channel_name, action) {
        (Some("bbo-tbt"), _) => LobEventKind::Bbo,
        (Some("books5"), _) => LobEventKind::Snapshot,
        (_, Some("snapshot")) => LobEventKind::Snapshot,
        (_, Some("update")) if bids_empty && asks_empty => LobEventKind::Heartbeat,
        (_, Some("update")) => LobEventKind::Incremental,
        _ => LobEventKind::Incremental,
    }
}

fn okx_lob_seq(prev: Option<i64>, seq: Option<i64>) -> Option<LobSeq> {
    if prev.is_none() && seq.is_none() {
        return None;
    }

    let prev = prev.and_then(non_negative_i64_to_u64);
    let seq = seq.and_then(non_negative_i64_to_u64);

    Some(LobSeq {
        prev,
        first: seq,
        last: seq,
    })
}

fn non_negative_i64_to_u64(value: i64) -> Option<u64> {
    if value >= 0 { Some(value as u64) } else { None }
}

#[cfg(test)]
mod tests {
    use crate::arch::{
        market_assets::exchange::okx::okx_ws_msg::OkxWsData,
        strategy_base::handler::lob_events::LobEventKind, traits::conversion::IntoWsData,
    };

    use super::*;

    #[test]
    fn parses_okx_books_snapshot() {
        let raw = r#"{
            "arg": {"channel": "books", "instId": "BTC-USDT-SWAP"},
            "action": "snapshot",
            "data": [{
                "asks": [["8476.98", "415", "0", "13"]],
                "bids": [["8476.97", "256", "0", "12"]],
                "ts": "1597026383085",
                "checksum": -855196043,
                "prevSeqId": -1,
                "seqId": 123456
            }]
        }"#;

        let parsed: OkxWsData<OkxWsLobBook> = serde_json::from_str(raw).unwrap();
        let lob = parsed.into_ws();

        assert_eq!(lob.len(), 1);
        assert!(matches!(lob[0].event, LobEventKind::Snapshot));
        assert_eq!(lob[0].inst, "BTC_USDT_PERP");
        assert_eq!(lob[0].seq.as_ref().unwrap().prev, None);
        assert_eq!(lob[0].seq.as_ref().unwrap().last, Some(123456));
        assert_eq!(lob[0].checksum.as_deref(), Some("-855196043"));
        assert_eq!(lob[0].asks[0].order_count, Some(13));
    }

    #[test]
    fn parses_okx_bbo_from_arg_inst() {
        let raw = r#"{
            "arg": {"channel": "bbo-tbt", "instId": "BCH-USDT-SWAP"},
            "data": [{
                "asks": [["111.06", "55154", "0", "2"]],
                "bids": [["111.05", "57745", "0", "2"]],
                "ts": "1670324386802",
                "seqId": 363996337
            }]
        }"#;

        let parsed: OkxWsData<OkxWsLobBook> = serde_json::from_str(raw).unwrap();
        let lob = parsed.into_ws();

        assert_eq!(lob.len(), 1);
        assert!(matches!(lob[0].event, LobEventKind::Bbo));
        assert_eq!(lob[0].inst, "BCH_USDT_PERP");
        assert_eq!(lob[0].seq.as_ref().unwrap().last, Some(363996337));
        assert_eq!(lob[0].bids[0].price, 111.05);
    }

    #[test]
    fn parses_okx_empty_update_as_heartbeat() {
        let raw = r#"{
            "arg": {"channel": "books", "instId": "BTC-USDT"},
            "action": "update",
            "data": [{
                "asks": [],
                "bids": [],
                "ts": "1597026383085",
                "prevSeqId": 15,
                "seqId": 15
            }]
        }"#;

        let parsed: OkxWsData<OkxWsLobBook> = serde_json::from_str(raw).unwrap();
        let lob = parsed.into_ws();

        assert_eq!(lob.len(), 1);
        assert!(matches!(lob[0].event, LobEventKind::Heartbeat));
        assert_eq!(lob[0].inst, "BTC_USDT");
        assert!(lob[0].bids.is_empty());
        assert!(lob[0].asks.is_empty());
    }

    #[test]
    fn okx_lob_event_messages_produce_no_data() {
        let raw = r#"{
            "event": "error",
            "code": "64003",
            "msg": "Only API users who are VIP4 and above are allowed.",
            "arg": {"channel": "books-l2-tbt", "instId": "BTC-USDT-SWAP"}
        }"#;

        let parsed: OkxWsData<OkxWsLobBook> = serde_json::from_str(raw).unwrap();
        assert!(parsed.into_ws().is_empty());
    }
}
