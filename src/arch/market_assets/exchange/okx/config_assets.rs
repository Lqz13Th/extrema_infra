/// OKX API Base
pub const OKX_WS_PUB: &str = "wss://ws.okx.com:8443/ws/v5/public";
pub const OKX_WS_PRI: &str = "wss://ws.okx.com:8443/ws/v5/private";
pub const OKX_WS_BUS: &str = "wss://ws.okx.com:8443/ws/v5/business";
pub const OKX_BASE_URL: &str = "https://www.okx.com";

/// REST endpoints
pub const OKX_ACCOUNT_BALANCE: &str = "/api/v5/account/balance";
pub const OKX_ACCOUNT_POSITIONS: &str = "/api/v5/account/positions";
pub const OKX_TRADE_ORDER: &str = "/api/v5/trade/order";
pub const OKX_TRADE_ORDERS_HISTORY: &str = "/api/v5/trade/orders-history";
pub const OKX_PUBLIC_INSTRUMENTS: &str = "/api/v5/public/instruments";
pub const OKX_PUBLIC_FUNDING_RATE: &str = "/api/v5/public/funding-rate";
pub const OKX_MARKET_TICKER: &str = "/api/v5/market/ticker";
pub const OKX_MARKET_CANDLES: &str = "/api/v5/market/candles";
pub const OKX_MARKET_BOOKS: &str = "/api/v5/market/books";
pub const OKX_CT_PUBLIC_LEADTRADERS: &str = "/api/v5/copytrading/public-lead-traders";
pub const OKX_CT_PUBLIC_LEADTRADER_STATS: &str = "/api/v5/copytrading/public-stats";
pub const OKX_CT_CURRENT_LEADTRADERS: &str = "/api/v5/copytrading/current-lead-traders";
pub const OKX_CT_LEADTRADER_SUBPOSITIONS: &str = "/api/v5/copytrading/public-current-subpositions";
pub const OKX_CT_LEADTRADER_SUBPOSITIONS_HISTORY: &str =
    "/api/v5/copytrading/public-subpositions-history";

/// WebSocket channels
pub const OKX_WS_LOGIN: &str = "GET/users/self/verify";
