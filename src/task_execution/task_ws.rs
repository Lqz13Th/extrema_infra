use crate::market_assets::market_core::Market;

#[derive(Clone, Debug)]
pub struct WsTaskInfo {
    pub market: Market,
    pub ws_channel: WsChannel,
    pub chunk: usize,
}

#[derive(Clone, Debug)]
pub struct WsSubscription {
    pub msg: Option<String>,
    pub url: String,
}

#[derive(Clone, Debug)]
pub enum WsChannel {
    Account,
    Candle(Option<CandleParam>),
    Trades(Option<TradesParam>),
    Tick,
    Lob,
    Other(String),
}

#[derive(Clone, Debug)]
pub enum CandleParam {
    OneSecond,
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    OneHour,
    FourHours,
    OneDay,
    OneWeek,
}

#[derive(Clone, Debug)]
pub enum TradesParam {
    AggTrades,
    Trades,
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
}