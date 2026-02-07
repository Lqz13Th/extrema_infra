use serde::{Deserialize, Serialize};

use crate::arch::market_assets::base_data::*;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BalanceData {
    pub timestamp: u64,
    pub asset: String,
    pub total: f64,
    pub frozen: f64,
    pub available: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PositionData {
    pub timestamp: u64,
    pub inst: String,
    pub inst_type: InstrumentType,
    pub position_side: PositionSide,
    pub size: f64,
    pub avg_price: f64,
    pub mark_price: f64,
    pub margin: f64,
    pub leverage: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BorrowableData {
    pub timestamp: u64,
    pub asset: String,
    pub available: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OrderAckData {
    pub timestamp: u64,
    pub order_status: OrderStatus,
    pub order_id: String,
    pub cli_order_id: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HistoOrderData {
    pub timestamp: u64,
    pub inst: String,
    pub order_id: String,
    pub cli_order_id: Option<String>,
    pub side: OrderSide,
    pub position_side: Option<PositionSide>,
    pub order_type: OrderType,
    pub order_status: OrderStatus,
    pub price: f64,
    pub avg_price: f64,
    pub size: f64,
    pub executed_size: f64,
    pub fee: Option<f64>,
    pub fee_currency: Option<String>,
    pub reduce_only: Option<bool>,
    pub time_in_force: Option<TimeInForce>,
    pub update_time: u64,
}
