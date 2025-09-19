use crate::market_assets::{
    market_core::Market,
    base_data::*
};
use crate::task_execution::task_ws::CandleParam;

#[derive(Clone, Debug)]
pub struct WsTrade {
    pub timestamp: u64,
    pub market: Market,
    pub inst: String,
    pub price: f64,
    pub size: f64,
    pub side: OrderSide,
    pub trade_id: u64,
}

#[derive(Clone, Debug)]
pub struct WsLob {
    pub timestamp: u64,
    pub market: Market,
    pub inst: String,
    pub bids: Vec<(f64, f64)>, // (price, size)
    pub asks: Vec<(f64, f64)>, // (price, size)
}

#[derive(Clone, Debug)]
pub struct WsCandle {
    pub timestamp: u64,
    pub market: Market,
    pub inst: String,
    pub interval: CandleParam,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub confirm: bool,
}

#[derive(Clone, Debug)]
pub struct WsAccOrder {
    pub timestamp: u64,
    pub market: Market,
    pub inst: String,
    pub inst_type: InstrumentType,
    pub price: f64,
    pub size: f64,
    pub filled_size: f64,
    pub side: OrderSide,
    pub status: OrderStatus,
    pub order_type: OrderType,
    pub client_order_id: Option<String>,
}


