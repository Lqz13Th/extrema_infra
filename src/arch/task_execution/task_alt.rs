use std::time::Duration;

/// Descriptor for a non-websocket runtime task.
///
/// Alt tasks cover scheduler ticks, order execution relays, instrument intents,
/// and model workers. `chunk` spawns multiple identical task instances. If
/// `task_base_id` is set, generated IDs are `base..base + chunk - 1`.
#[derive(Clone, Debug)]
pub struct AltTaskInfo {
    /// Kind of alt task to spawn.
    pub alt_task_type: AltTaskType,
    /// Number of task instances to spawn.
    pub chunk: u64,
    /// Optional first task id for generated task instances.
    pub task_base_id: Option<u64>,
}

/// Built-in non-websocket task kinds.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AltTaskType {
    /// Order execution task.
    OrderExecution,
    /// Instrument, allocation, or portfolio intent task.
    InstIntent,
    /// Model prediction worker.
    #[cfg(any(feature = "model_onnx", feature = "model_zmq"))]
    ModelPreds(ModelRunner),
    /// Periodic scheduler task.
    TimeScheduler(Duration),
}

/// Supported model worker backends.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ModelRunner {
    /// External model worker reached over ZeroMQ.
    #[cfg(feature = "model_zmq")]
    Zmq(u64),
    /// In-process ONNX model loaded from a path or JSON config.
    #[cfg(feature = "model_onnx")]
    Onnx(String),
}
