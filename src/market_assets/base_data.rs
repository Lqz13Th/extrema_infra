#[derive(Debug, Clone,PartialEq, Eq, Hash, Default)]
pub enum Market {
    #[default]
    BinanceUmFutures,
    BinanceSpot,
    Coinbase,
    OkxSwap,
    SolPumpFun,
}

#[derive(Debug, Clone,PartialEq, Eq, Hash, Default)]
pub enum SymbolStatus {
    #[default]
    Live,
    Suspend,
    PreOpen,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    BUY,
    SELL,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PositionSide {
    Long,
    Short,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarginMode {
    Cross,
    Isolated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderType {
    Market,
    Limit,
    Trigger,
    PostOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimeInForce {
    GTC,
    IOC,
    FOK,
    GTD,
}

pub const PERPETUAL: &str = "PERPETUAL";
pub const TRADING: &str = "TRADING";
pub const SUBSCRIBE: &str = "SUBSCRIBE";