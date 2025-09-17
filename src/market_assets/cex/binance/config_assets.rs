/// # Spot API
pub const BINANCE_SPOT_BASE_URL: &str = "https://api1.binance.com";
pub const BINANCE_SPOT_EXCHANGE_INFO: &str = "/api/v3/exchangeInfo";
pub const BINANCE_SPOT_ACCOUNT_INFO: &str = "/api/v3/account";
pub const BINANCE_SPOT_MY_TRADES: &str = "/api/v3/myTrades";
pub const SPOT_USER_DATA_STREAM: &str = "/api/v3/userDataStream";

/// # UmFutures API
pub const BINANCE_UM_FUTURES_WS: &str = "wss://fstream.binance.com/ws";
pub const BINANCE_UM_FUTURES_BASE_URL: &str = "https://fapi.binance.com";
pub const BINANCE_UM_FUTURES_EXCHANGE_INFO: &str = "/fapi/v1/exchangeInfo";
pub const BINANCE_UM_FUTURES_ACCOUNT_INFO: &str = "/fapi/v3/account";
pub const BINANCE_UM_FUTURES_BALANCE_INFO: &str = "/fapi/v3/balance";
pub const BINANCE_UM_FUTURES_LISTEN_KEY: &str = "/fapi/v1/listenKey";

/// # CmFutures API
pub const BINANCE_CM_FUTURES_BASE_URL: &str = "https://dapi.binance.com";


/// # Websocket subscription
pub const BINANCE_CANDLE_SUBSCRIPTIONS: [&str; 8] = [
    "kline_1s",
    "kline_1m",
    "Kline_5m",
    "Kline_15m",
    "Kline_1h",
    "Kline_4h",
    "Kline_1d",
    "Kline_1w",
];
