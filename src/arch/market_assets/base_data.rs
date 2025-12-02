use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum InstrumentType {
    Spot,
    Perpetual,
    Futures,
    Options,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum InstrumentStatus {
    Live,
    Suspend,
    PreOpen,
    Closed,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum OrderSide {
    BUY,
    SELL,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum OrderStatus {
    Live,
    PartiallyFilled,
    Filled,
    Expired,
    Canceled,
    Rejected,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
    PostOnly,
    Fok,
    Ioc,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum MarginMode {
    Cross,
    Isolated,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum PositionSide {
    Long,
    Short,
    Both,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
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