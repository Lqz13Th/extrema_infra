use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};

use crate::arch::market_assets::api_general::OrderParams;
use crate::prelude::Market;

#[derive(Clone, Debug)]
pub struct AltScheduleEvent {
    pub timestamp: u64,
    pub duration: Duration,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AltTensor {
    pub timestamp: u64,                    // Timestamp of the data
    pub data: Vec<f32>,                    // Flattened N-dimensional array stored as a 1D vector
    pub shape: Vec<usize>, // Shape of the tensor, length = number of dimensions (N-D)
    pub metadata: HashMap<String, String>, // model, instrument, threshold, etc
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AltOrder {
    pub timestamp: u64,
    pub market: Market,
    pub order_info: HashMap<String, String>,
    pub order_params: OrderParams,
}
