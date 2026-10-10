use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};

use crate::arch::market_assets::{
    api_general::OrderParams, base_data::InstrumentKey, market_core::Market,
};

/// Scheduler tick delivered to `EventHandler::on_schedule`.
#[derive(Clone, Debug)]
pub struct AltScheduleEvent {
    /// Tick time in microseconds.
    pub timestamp: u64,
    /// Interval configured on the `TimeScheduler` task.
    pub duration: Duration,
}

/// Generic dense tensor payload exchanged across alt feature/model channels.
///
/// Contract:
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
    /// Timestamp of the data in microseconds.
    pub timestamp: u64,
    /// Flattened tensor values in row-major order.
    pub data: Vec<f32>,
    /// Tensor shape before flattening; its length is the number of dimensions.
    pub shape: Vec<usize>,
    /// Free-form labels such as model name, instrument or threshold. The ONNX
    /// runner adds `model_name` and `output_index` to its predictions.
    pub metadata: HashMap<String, String>,
}

/// One order in an `OrderExecute` batch, delivered to
/// `EventHandler::on_order_execution`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AltOrder {
    pub timestamp: u64,
    pub market: Market,
    pub order_params: OrderParams,
    pub metadata: HashMap<String, String>,
}

/// Instrument-level targets published with `TaskCommand::InstIntent` and
/// delivered to `EventHandler::on_inst_intent`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AltIntent {
    pub timestamp: u64,
    pub intent_type: IntentType,
    pub intents: HashMap<InstrumentKey, f64>,
    pub metadata: HashMap<String, String>,
}

/// Meaning of the values in [`AltIntent::intents`].
#[derive(Clone, Debug, Default, PartialEq)]
pub enum IntentType {
    /// Target weights.
    #[default]
    Weight,
    /// Target prices.
    Price,
}
