use crate::arch::market_assets::market_core::Market;

/// Descriptor for a websocket relay task.
///
/// The relay owns websocket IO for a market/channel pair and publishes
/// normalized events into matching broadcast channels. Strategies usually react
/// to the initial `on_ws_event` event by sending connect/login/subscribe
/// commands through the task handle.
#[derive(Clone, Debug)]
pub struct WsTaskInfo {
    /// Exchange or venue for this websocket task.
    pub market: Market,
    /// Stream category handled by this task.
    pub ws_channel: WsChannel,
    /// Whether parse failures from expected non-target websocket payloads should
    /// be ignored quietly instead of logged as parse errors.
    pub filter_channels: bool,
    /// Number of task instances to spawn.
    pub chunk: u64,
    /// Optional first task id for generated task instances.
    pub task_base_id: Option<u64>,
}

/// Websocket channel categories used by websocket task declarations.
///
/// A variant is usable only when the selected exchange client and relay routing
/// implement that market/channel combination.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum WsChannel {
    /// Private account order updates.
    AccountOrders,
    /// Private balance and position updates.
    AccountBalAndPos,
    /// Private position-only updates.
    AccountPositions,
    /// Public candles, optionally parameterized by interval.
    Candles(Option<CandleParam>),
    /// Public trades, optionally parameterized by trade stream type.
    Trades(Option<TradesParam>),
    /// Public ticker stream.
    Tick,
    /// Public order book stream.
    Lob,
    /// Public market-by-order order book stream.
    LobMbo,
    /// Exchange-specific or custom stream.
    Other(String),
}

/// Candle interval used by candle websocket and REST APIs.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CandleParam {
    OneSecond,
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    OneHour,
    FourHours,
    OneDay,
    OneWeek,
    Custom(String),
}

/// Trade stream variant for exchanges that expose several trade feeds.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TradesParam {
    AggTrades,
    AllTrades,
}

impl CandleParam {
    /// Parses a standard candle interval such as `"1m"` or `"1h"`.
    pub fn from_candle_str(s: &str) -> Option<Self> {
        match s {
            "1s" => Some(CandleParam::OneSecond),
            "1m" => Some(CandleParam::OneMinute),
            "5m" => Some(CandleParam::FiveMinutes),
            "15m" => Some(CandleParam::FifteenMinutes),
            "1h" => Some(CandleParam::OneHour),
            "4h" => Some(CandleParam::FourHours),
            "1d" => Some(CandleParam::OneDay),
            "1w" => Some(CandleParam::OneWeek),
            _ => None,
        }
    }

    /// Returns the exchange-style interval string.
    pub fn as_str(&self) -> &str {
        match self {
            CandleParam::OneSecond => "1s",
            CandleParam::OneMinute => "1m",
            CandleParam::FiveMinutes => "5m",
            CandleParam::FifteenMinutes => "15m",
            CandleParam::OneHour => "1h",
            CandleParam::FourHours => "4h",
            CandleParam::OneDay => "1d",
            CandleParam::OneWeek => "1w",
            CandleParam::Custom(s) => s.as_str(),
        }
    }
}
