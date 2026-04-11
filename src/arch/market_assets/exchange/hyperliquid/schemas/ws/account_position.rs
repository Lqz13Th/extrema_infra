use serde::Deserialize;

use crate::arch::{
    market_assets::exchange::hyperliquid::{
        hyperliquid_ws_msg::HyperliquidWsEvent,
        schemas::rest::clearinghouse_state::RestClearinghouseStateHyperliquid,
    },
    strategy_base::handler::lob_events::WsAccPosition,
    traits::conversion::IntoWsData,
};

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum WsAccountPositionMsgHyperliquid {
    Channel(WsAccountPositionChannelHyperliquid),
    Event(HyperliquidWsEvent),
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsAccountPositionChannelHyperliquid {
    pub data: RestClearinghouseStateHyperliquid,
}

impl IntoWsData for WsAccountPositionMsgHyperliquid {
    type Output = Vec<WsAccPosition>;

    fn into_ws(self) -> Self::Output {
        match self {
            Self::Channel(c) => c
                .data
                .assetPositions
                .into_iter()
                .map(|position| position.into_ws_position())
                .collect(),
            Self::Event(_) => Vec::new(),
        }
    }
}
