use crate::arch::market_assets::market_core::Market;

#[derive(Clone, Debug)]
pub struct WsTaskInfo {
    pub market: Market,
    pub ws_channel: WsChannel,
    pub filter_channels: bool,
    pub chunk: u64,
    pub task_id: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WsChannel {
    AccountOrders,
    AccountPositions,
    AccountBalAndPos,
    Candles(Option<CandleParam>),
    Trades(Option<TradesParam>),
    Tick,
    Lob,
    Other(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum CandleParam {
    OneSecond,
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    OneHour,
    FourHours,
    OneDay,
    OneWeek,
    Custom(String)
}

#[derive(Clone, Debug, PartialEq)]
pub enum TradesParam {
    AggTrades,
    AllTrades,
}



impl CandleParam {
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