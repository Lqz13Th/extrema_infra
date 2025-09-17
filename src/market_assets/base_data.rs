use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InstrumentType {
    Spot,
    Perpetual,
    Option,
}

#[derive(Clone, Debug, Default)]
pub enum SymbolStatus {
    #[default]
    Live,
    Suspend,
    PreOpen,
    Closed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OrderStatus {
    Live,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OrderSide {
    BUY,
    SELL,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PositionSide {
    Long,
    Short,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MarginMode {
    Cross,
    Isolated,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
    PostOnly,
    Fok,
    Ioc,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TimeInForce {
    GTC,
    IOC,
    FOK,
    GTD,
}

pub const PERPETUAL: &str = "PERPETUAL";
pub const TRADING: &str = "TRADING";
pub const SUBSCRIBE: &str = "SUBSCRIBE";