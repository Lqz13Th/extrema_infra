/// OKX API Base
pub const OKX_BASE_URL: &str = "https://www.okx.com";

/// REST endpoints
pub const OKX_ACCOUNT_BALANCE: &str = "/api/v5/account/balance";
pub const OKX_ORDER: &str = "/api/v5/trade/order";
pub const OKX_ORDERS_HISTORY: &str = "/api/v5/trade/orders-history";
pub const OKX_INSTRUMENTS: &str = "/api/v5/public/instruments";
pub const OKX_TICKER: &str = "/api/v5/market/ticker";
pub const OKX_CANDLE: &str = "/api/v5/market/candles";
pub const OKX_ORDER_BOOK: &str = "/api/v5/market/books";

/// WebSocket channels
pub const OKX_WS_SUBSCRIPTIONS: [&str; 10] = [
    "spot/trade",           // Spot 成交
    "spot/depth",           // Spot 深度
    "spot/candle1m",        // Spot K线
    "swap/trade",           // 永续合约成交
    "swap/depth",           // 永续合约深度
    "swap/candle1m",        // 永续合约K线
    "futures/trade",        // 交割合约成交
    "futures/depth",        // 交割合约深度
    "futures/candle1m",     // 交割合约K线
    "account/account"       // 账户资金变动
];
