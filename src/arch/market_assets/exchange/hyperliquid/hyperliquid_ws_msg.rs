use serde::Deserialize;
use serde_json::Value;
use tracing::info;

use crate::arch::traits::conversion::IntoWsData;

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum HyperliquidWsData<T> {
    Channel(HyperliquidWsChannel<T>),
    Event(HyperliquidWsEvent),
}

#[derive(Clone, Debug, Deserialize)]
pub struct HyperliquidWsChannel<T> {
    pub channel: String,
    pub data: HyperliquidWsState<T>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum HyperliquidWsState<T> {
    Batch(Vec<T>),
    Clearinghouse(WsAccountPositionMsgHyperliquid<T>),
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct WsAccountPositionMsgHyperliquid<T> {
    #[serde(default)]
    pub dex: String,
    #[serde(default)]
    pub user: String,
    pub clearinghouseState: WsClearinghouseStateHyperliquid<T>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct WsClearinghouseStateHyperliquid<T> {
    pub assetPositions: Vec<T>,
    pub time: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HyperliquidWsEvent {
    pub channel: Option<String>,
    pub data: Option<Value>,
}

impl<T> IntoWsData for HyperliquidWsData<T>
where
    T: IntoWsData + for<'de> Deserialize<'de>,
{
    type Output = Vec<T::Output>;

    fn into_ws(self) -> Self::Output {
        match self {
            HyperliquidWsData::Channel(c) => match c.data {
                HyperliquidWsState::Batch(data) => data.into_iter().map(|d| d.into_ws()).collect(),
                HyperliquidWsState::Clearinghouse(msg) => msg
                    .clearinghouseState
                    .assetPositions
                    .into_iter()
                    .map(|position| position.into_ws())
                    .collect(),
            },
            HyperliquidWsData::Event(event) => {
                if !matches!(event.channel.as_deref(), Some("pong")) {
                    info!(
                        "Hyperliquid WS event: channel = {:?}, data = {:?}",
                        event.channel, event.data
                    );
                }
                Vec::new()
            },
        }
    }
}
