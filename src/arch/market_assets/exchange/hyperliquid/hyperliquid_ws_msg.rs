use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use tracing::info;

use crate::arch::{
    task_execution::ws_runner::ws_decode::decode_preferred, traits::conversion::IntoWsData,
};

use super::schemas::ws::lob::WsLobHyperliquid;

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

#[derive(Deserialize)]
struct HyperliquidWsTypedChannel<T> {
    channel: String,
    data: T,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum HyperliquidWsState<T> {
    ChannelBatch(Vec<T>),
    Clearinghouse(WsAccountPositionMsgHyperliquid<T>),
    ChannelSingle(T),
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

impl<T: DeserializeOwned> HyperliquidWsData<T> {
    pub(crate) fn decode_batch(frame: &[u8]) -> serde_json::Result<Self> {
        Self::decode_state(frame, HyperliquidWsState::ChannelBatch)
    }

    pub(crate) fn decode_clearinghouse(frame: &[u8]) -> serde_json::Result<Self> {
        Self::decode_state(frame, HyperliquidWsState::Clearinghouse)
    }

    fn decode_single_with<Message, Wrap>(frame: &[u8], wrap: Wrap) -> serde_json::Result<Self>
    where
        Message: DeserializeOwned,
        Wrap: FnOnce(Message) -> T,
    {
        Self::decode_state(frame, |message| {
            HyperliquidWsState::ChannelSingle(wrap(message))
        })
    }

    fn decode_state<Message, Wrap>(frame: &[u8], wrap: Wrap) -> serde_json::Result<Self>
    where
        Message: DeserializeOwned,
        Wrap: FnOnce(Message) -> HyperliquidWsState<T>,
    {
        decode_preferred(frame, |message: HyperliquidWsTypedChannel<Message>| {
            Self::Channel(HyperliquidWsChannel {
                channel: message.channel,
                data: wrap(message.data),
            })
        })
    }
}

impl HyperliquidWsData<WsLobHyperliquid> {
    pub(crate) fn decode_l2_book(frame: &[u8]) -> serde_json::Result<Self> {
        Self::decode_single_with(frame, WsLobHyperliquid::Book)
    }

    pub(crate) fn decode_bbo(frame: &[u8]) -> serde_json::Result<Self> {
        Self::decode_single_with(frame, WsLobHyperliquid::Bbo)
    }
}

impl<T> IntoWsData for HyperliquidWsData<T>
where
    T: IntoWsData + for<'de> Deserialize<'de>,
{
    type Output = Vec<T::Output>;

    fn into_ws(self) -> Self::Output {
        match self {
            HyperliquidWsData::Channel(c) => match c.data {
                HyperliquidWsState::ChannelBatch(data) => {
                    data.into_iter().map(|d| d.into_ws()).collect()
                },
                HyperliquidWsState::Clearinghouse(msg) => msg
                    .clearinghouseState
                    .assetPositions
                    .into_iter()
                    .map(|position| position.into_ws())
                    .collect(),
                HyperliquidWsState::ChannelSingle(data) => vec![data.into_ws()],
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
