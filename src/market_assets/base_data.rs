use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq)]
pub enum InstrumentType {
    Spot,
    Perpetual,
    Futures,
    Option,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum InstrumentStatus {
    Live,
    Suspend,
    PreOpen,
    Closed,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum OrderStatus {
    Live,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum OrderSide {
    BUY,
    SELL,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum PositionSide {
    Long,
    Short,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum MarginMode {
    Cross,
    Isolated,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum OrderType {
    Market,
    Limit,
    PostOnly,
    Fok,
    Ioc,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum TimeInForce {
    GTC,
    IOC,
    FOK,
    GTD,
    #[default]
    Unknown,
}

pub const PERPETUAL: &str = "PERPETUAL";
pub const TRADING: &str = "TRADING";
pub const SUBSCRIBE: &str = "SUBSCRIBE";
pub const SUBSCRIBE_LOWER: &str = "subscribe";