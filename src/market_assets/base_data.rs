use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InstrumentType {
    Spot,
    Perpetual,
    Option,
    Unknown,
}

#[derive(Clone, Debug, Default)]
pub enum InstrumentStatus {
    #[default]
    Live,
    Suspend,
    PreOpen,
    Closed,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OrderStatus {
    Live,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OrderSide {
    BUY,
    SELL,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PositionSide {
    Long,
    Short,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MarginMode {
    Cross,
    Isolated,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
    PostOnly,
    Fok,
    Ioc,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TimeInForce {
    GTC,
    IOC,
    FOK,
    GTD,
    Unknown,
}

pub const PERPETUAL: &str = "PERPETUAL";
pub const TRADING: &str = "TRADING";
pub const SUBSCRIBE: &str = "SUBSCRIBE";
pub const SUBSCRIBE_LOWER: &str = "subscribe";