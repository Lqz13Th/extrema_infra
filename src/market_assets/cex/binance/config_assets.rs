/// # Spot API
pub const SPOT_BASE_URL: &str = "https://api1.binance.com";
pub const SPOT_EXCHANGE_INFO: &str = "/api/v3/exchangeInfo";
pub const SPOT_ACCOUNT_INFO: &str = "/api/v3/account";
pub const SPOT_MY_TRADES: &str = "/api/v3/myTrades";
pub const SPOT_USER_DATA_STREAM: &str = "/api/v3/userDataStream";

/// # UmFutures API
pub const UM_FUTURES_WS: &str = "wss://fstream.binance.com/ws";
pub const UM_FUTURES_BASE_URL: &str = "https://fapi.binance.com";
pub const UM_FUTURES_EXCHANGE_INFO: &str = "/fapi/v1/exchangeInfo";
pub const UM_FUTURES_ACCOUNT_INFO: &str = "/fapi/v3/account";
pub const UM_FUTURES_BALANCE_INFO: &str = "/fapi/v3/balance";
pub const UM_FUTURES_LISTEN_KEY: &str = "/fapi/v1/listenKey";

/// # CmFutures API
pub const CM_FUTURES_BASE_URL: &str = "https://dapi.binance.com";


pub const FUTURE_CANDLE_SUBSCRIPTIONS: [&str; 8] = [
    "continuousKline_1s",
    "continuousKline_1m",
    "continuousKline_5m",
    "continuousKline_15m",
    "continuousKline_1h",
    "continuousKline_4h",
    "continuousKline_1d",
    "continuousKline_1w",
];
