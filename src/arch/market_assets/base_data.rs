use serde::{Deserialize, Serialize};

use super::market_core::{Market, MarketScope};

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct InstrumentKey {
    pub market: Option<Market>,
    pub inst_type: InstrumentType,
    pub inst: String,
    pub extra: Option<String>,
}

impl InstrumentKey {
    pub fn market_scope(&self) -> Option<MarketScope> {
        self.market
            .clone()
            .map(|market| MarketScope::new(market, self.extra.clone()))
    }
}

#[derive(Clone, Debug, Default, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum InstrumentType {
    Spot,
    Perpetual,
    Futures,
    Options,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum InstrumentStatus {
    Live,
    Suspend,
    PreOpen,
    Delisting,
    Closed,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum OrderSide {
    BUY,
    SELL,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, Hash, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Default, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
    PostOnly,
    Fok,
    Ioc,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum MarginMode {
    Cross,
    Isolated,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum PositionSide {
    Long,
    Short,
    Both,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, Hash, Eq, PartialEq, Serialize, Deserialize)]
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
pub const TRADING_LOWER: &str = "trading";
pub const SUBSCRIBE: &str = "SUBSCRIBE";
pub const SUBSCRIBE_LOWER: &str = "subscribe";
