use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};

use crate::arch::market_assets::{
    api_general::OrderParams, base_data::InstrumentKey, market_core::Market,
};

#[derive(Clone, Debug)]
pub struct AltScheduleEvent {
    pub timestamp: u64,
    pub duration: Duration,
}

/// Generic dense tensor payload exchanged across alt feature/model channels.
///
/// Contract v1:
/// - `data` stores a row-major / C-order flatten view of the tensor.
/// - `shape` stores the original tensor shape before flattening.
/// - `data.len()` must equal the product of all entries in `shape`.
/// - No implicit transpose / squeeze / reshape is performed by infra.
/// - For tensors that were permuted in PyTorch, materialize them as contiguous
///   before flattening (`tensor.contiguous().view(-1)` semantics).
///
/// Feature input examples:
/// - tabular single row: `shape=[1, 4]`, `data=[f1, f2, f3, f4]`
/// - multi-asset single row: `shape=[1, 3, 5]`, `data.len() == 15`
/// - conv input (NCHW): `shape=[1, 1, 3, 3]`, `data.len() == 9`
///
/// Model output examples:
/// - scalar / regression: `shape=[1, 1]`, `data=[173.37]`
/// - class probabilities: `shape=[1, 3]`, `data=[0.97, 0.015, 0.015]`
/// - conv feature map: `shape=[1, 16, 8, 8]`, `data.len() == 1024`
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AltTensor {
    pub timestamp: u64,                    // Timestamp of the data
    pub data: Vec<f32>,                    // Flattened N-dimensional array stored as a 1D vector
    pub shape: Vec<usize>, // Shape of the tensor, length = number of dimensions (N-D)
    pub metadata: HashMap<String, String>, // model, instrument, threshold, etc
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AltOrder {
    pub timestamp: u64,
    pub market: Market,
    pub order_params: OrderParams,
    pub metadata: HashMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AltIntent {
    pub timestamp: u64,
    pub intent_type: IntentType,
    pub intents: HashMap<InstrumentKey, f64>,
    pub metadata: HashMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum IntentType {
    #[default]
    Weight,
    Price,
}
