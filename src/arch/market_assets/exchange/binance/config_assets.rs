/// Spot API
pub const BINANCE_SPOT_BASE_URL: &str = "https://api1.binance.com";
pub const BINANCE_SPOT_WS_API: &str = "wss://ws-api.binance.com:443/ws-api/v3";
pub const BINANCE_SPOT_EXCHANGE_INFO: &str = "/api/v3/exchangeInfo";
pub const BINANCE_SPOT_TICKERS: &str = "/api/v3/ticker/price";
pub const BINANCE_SPOT_PLACE_ORDER: &str = "/api/v3/order";
pub const BINANCE_SPOT_CANCEL_ORDER: &str = "/api/v3/order";
pub const BINANCE_SPOT_ACCOUNT_INFO: &str = "/api/v3/account";
pub const BINANCE_SPOT_MY_TRADES: &str = "/api/v3/myTrades";
pub const SPOT_USER_DATA_STREAM: &str = "/api/v3/userDataStream";
pub const BINANCE_USER_UNIVERSAL_TRANSFER: &str = "/sapi/v1/asset/transfer";
pub const BINANCE_WITHDRAW_APPLY: &str = "/sapi/v1/capital/withdraw/apply";

/// UmFutures API
pub const BINANCE_UM_FUTURES_WS: &str = "wss://fstream.binance.com/ws";
pub const BINANCE_UM_FUTURES_BASE_URL: &str = "https://fapi.binance.com";
pub const BINANCE_UM_FUTURES_EXCHANGE_INFO: &str = "/fapi/v1/exchangeInfo";
pub const BINANCE_UM_FUTURES_ACCOUNT_INFO: &str = "/fapi/v3/account";
pub const BINANCE_UM_FUTURES_BALANCE_INFO: &str = "/fapi/v3/balance";
pub const BINANCE_UM_FUTURES_PLACE_ORDER_INFO: &str = "/fapi/v1/order";
pub const BINANCE_UM_FUTURES_CHANGE_LEVERAGE: &str = "/fapi/v1/leverage";
pub const BINANCE_UM_FUTURES_POSITION_RISK_INFO: &str = "/fapi/v3/positionRisk";
pub const BINANCE_UM_FUTURES_TICKERS: &str = "/fapi/v2/ticker/price";
pub const BINANCE_UM_FUTURES_PREMIUM_INDEX_KLINES: &str = "/fapi/v1/premiumIndexKlines";
pub const BINANCE_UM_FUTURES_PREMIUM_INDEX: &str = "/fapi/v1/premiumIndex";
pub const BINANCE_UM_FUTURES_FUNDING_INFO: &str = "/fapi/v1/fundingInfo";
pub const BINANCE_UM_FUTURES_LISTEN_KEY: &str = "/fapi/v1/listenKey";
pub const BINANCE_UM_FUTURES_ALL_ORDERS: &str = "/fapi/v1/allOrders";

/// CmFutures API
pub const BINANCE_CM_FUTURES_WS: &str = "wss://dstream.binance.com/ws";
pub const BINANCE_CM_FUTURES_BASE_URL: &str = "https://dapi.binance.com";
pub const BINANCE_CM_FUTURES_EXCHANGE_INFO: &str = "/dapi/v1/exchangeInfo";

pub const BINANCE_CM_FUTURES_ACCOUNT_INFO: &str = "/dapi/v1/balance";
pub const BINANCE_CM_FUTURES_BALANCE_INFO: &str = "/dapi/v1/account";
pub const BINANCE_CM_FUTURES_LISTEN_KEY: &str = "/dapi/v1/listenKey";
