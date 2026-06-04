use serde::Deserialize;

use crate::arch::{
    market_assets::{
        api_general::ts_to_micros, exchange::hyperliquid::api_utils::hyperliquid_inst_to_cli,
        market_core::Market,
    },
    strategy_base::handler::lob_events::{LobEventKind, LobLevel, LobLevelAction, WsLob},
    traits::conversion::IntoWsData,
};

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum WsLobHyperliquid {
    Book(WsBookHyperliquid),
    Bbo(WsBboHyperliquid),
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsBookHyperliquid {
    coin: String,
    levels: Vec<Vec<WsLevelHyperliquid>>,
    time: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsBboHyperliquid {
    coin: String,
    bbo: Vec<Option<WsLevelHyperliquid>>,
    time: u64,
}

#[derive(Clone, Debug, Deserialize)]
struct WsLevelHyperliquid {
    px: String,
    sz: String,
    n: u64,
}

impl IntoWsData for WsLobHyperliquid {
    type Output = WsLob;

    fn into_ws(self) -> Self::Output {
        match self {
            WsLobHyperliquid::Book(book) => book.into_ws_lob(),
            WsLobHyperliquid::Bbo(bbo) => bbo.into_ws_lob(),
        }
    }
}

impl WsBookHyperliquid {
    fn into_ws_lob(self) -> WsLob {
        let mut levels = self.levels.into_iter();
        let bids = levels.next().unwrap_or_default();
        let asks = levels.next().unwrap_or_default();

        WsLob {
            timestamp: ts_to_micros(self.time),
            market: Market::HyperLiquid,
            inst: hyperliquid_inst_to_cli(&self.coin),
            event: LobEventKind::Snapshot,
            bids: bids.into_iter().map(Into::into).collect(),
            asks: asks.into_iter().map(Into::into).collect(),
            seq: None,
            checksum: None,
        }
    }
}

impl WsBboHyperliquid {
    fn into_ws_lob(self) -> WsLob {
        let bid = self.bbo.first().cloned().flatten();
        let ask = self.bbo.get(1).cloned().flatten();

        WsLob {
            timestamp: ts_to_micros(self.time),
            market: Market::HyperLiquid,
            inst: hyperliquid_inst_to_cli(&self.coin),
            event: LobEventKind::Bbo,
            bids: bid.into_iter().map(Into::into).collect(),
            asks: ask.into_iter().map(Into::into).collect(),
            seq: None,
            checksum: None,
        }
    }
}

impl From<WsLevelHyperliquid> for LobLevel {
    fn from(level: WsLevelHyperliquid) -> Self {
        LobLevel {
            price: level.px.parse().unwrap_or_default(),
            size: level.sz.parse().unwrap_or_default(),
            action: LobLevelAction::Upsert,
            order_count: Some(level.n),
            level_update_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::arch::{
        market_assets::exchange::hyperliquid::hyperliquid_ws_msg::HyperliquidWsData,
        strategy_base::handler::lob_events::LobEventKind, traits::conversion::IntoWsData,
    };

    use super::*;

    #[test]
    fn parses_l2_book_payload() {
        let raw = r#"{
            "channel": "l2Book",
            "data": {
                "coin": "BTC",
                "time": 1780540000123,
                "levels": [
                    [{"px": "63900.0", "sz": "1.25", "n": 3}],
                    [{"px": "63901.0", "sz": "0.75", "n": 2}]
                ]
            }
        }"#;

        let parsed: HyperliquidWsData<WsLobHyperliquid> = serde_json::from_str(raw).unwrap();
        let lob = parsed.into_ws();

        assert_eq!(lob.len(), 1);
        assert!(matches!(lob[0].event, LobEventKind::Snapshot));
        assert_eq!(lob[0].inst, "BTC_USDC_PERP");
        assert_eq!(lob[0].bids[0].price, 63900.0);
        assert_eq!(lob[0].bids[0].order_count, Some(3));
        assert_eq!(lob[0].asks[0].price, 63901.0);
        assert_eq!(lob[0].asks[0].order_count, Some(2));
    }

    #[test]
    fn parses_bbo_payload() {
        let raw = r#"{
            "channel": "bbo",
            "data": {
                "coin": "BTC",
                "time": 1780540000456,
                "bbo": [
                    {"px": "63899.0", "sz": "0.5", "n": 1},
                    {"px": "63900.0", "sz": "0.8", "n": 4}
                ]
            }
        }"#;

        let parsed: HyperliquidWsData<WsLobHyperliquid> = serde_json::from_str(raw).unwrap();
        let lob = parsed.into_ws();

        assert_eq!(lob.len(), 1);
        assert!(matches!(lob[0].event, LobEventKind::Bbo));
        assert_eq!(lob[0].bids[0].price, 63899.0);
        assert_eq!(lob[0].bids[0].order_count, Some(1));
        assert_eq!(lob[0].asks[0].price, 63900.0);
        assert_eq!(lob[0].asks[0].order_count, Some(4));
    }
}
