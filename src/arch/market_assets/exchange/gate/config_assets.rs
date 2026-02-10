/// Gate API Base
pub const GATE_BASE_URL: &str = "https://api.gateio.ws";
pub const GATE_WS_BASE_URL: &str = "wss://api.gateio.ws/ws/v4";
pub const GATE_FUTURES_WS_USDT: &str = "wss://fx-ws.gateio.ws/v4/ws/usdt";
pub const GATE_FUTURES_WS_BTC: &str = "wss://fx-ws.gateio.ws/v4/ws/btc";

/// Spot endpoints/channels
pub const GATE_WS_SPOT_ORDERS: &str = "spot.orders";
pub const GATE_WS_SPOT_BALANCES: &str = "spot.balances";
pub const GATE_WS_SPOT_CROSS_BALANCES: &str = "spot.cross_balances";
pub const GATE_SPOT_ORDERS: &str = "/api/v4/spot/orders";

/// Uni (margin/unified) REST endpoints
pub const GATE_UNI_MARGIN_CURRENCY_PAIRS: &str = "/api/v4/margin/uni/currency_pairs";
pub const GATE_UNI_MARGIN_ESTIMATE_RATE: &str = "/api/v4/margin/uni/estimate_rate";
pub const GATE_UNI_MARGIN_LOANS: &str = "/api/v4/margin/uni/loans";
pub const GATE_UNI_MARGIN_USER_ACCOUNT: &str = "/api/v4/margin/user/account";
pub const GATE_UNI_MARGIN_INTEREST_RECORDS: &str = "/api/v4/margin/uni/interest_records";
pub const GATE_UNI_MARGIN_AUTO_REPAY: &str = "/api/v4/margin/auto_repay";
pub const GATE_UNI_MARGIN_ACCOUNT_BOOK: &str = "/api/v4/margin/account_book";
pub const GATE_UNI_ACCOUNTS: &str = "/api/v4/unified/accounts";
pub const GATE_UNI_BORROWABLE: &str = "/api/v4/unified/borrowable";
pub const GATE_UNI_SUB_ACCOUNTS: &str = "/api/v4/sub_accounts";

/// Futures (perp) REST endpoints
pub const GATE_FUTURES_CONTRACTS: &str = "/api/v4/futures/{settle}/contracts";
pub const GATE_FUTURES_CONTRACT: &str = "/api/v4/futures/{settle}/contracts/{contract}";
pub const GATE_FUTURES_PREMIUM_INDEX: &str = "/api/v4/futures/{settle}/premium_index";
pub const GATE_FUTURES_FUNDING_RATE: &str = "/api/v4/futures/{settle}/funding_rate";
pub const GATE_FUTURES_SET_POSITION_MODE: &str = "/api/v4/futures/{settle}/set_position_mode";
pub const GATE_FUTURES_ORDERS: &str = "/api/v4/futures/{settle}/orders";
pub const GATE_WS_FUTURES_ORDERS: &str = "futures.orders";
pub const GATE_WS_FUTURES_BALANCES: &str = "futures.balances";
pub const GATE_WS_FUTURES_POSITIONS: &str = "futures.positions";
pub const GATE_WS_FUTURES_TRADES: &str = "futures.trades";
pub const GATE_WS_FUTURES_CANDLES: &str = "futures.candlesticks";

/// Delivery REST endpoints
pub const GATE_DELIVERY_CONTRACTS: &str = "/api/v4/delivery/{settle}/contracts";
pub const GATE_DELIVERY_CONTRACT: &str = "/api/v4/delivery/{settle}/contracts/{contract}";

/// Account REST endpoints
pub const GATE_ACCOUNT_DETAIL: &str = "/api/v4/account/detail";
pub const GATE_ACCOUNT_MAIN_KEYS: &str = "/api/v4/account/main_keys";
