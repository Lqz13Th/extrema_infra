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
    ModelPreds(ModelRunner),
    /// Periodic scheduler task.
    TimeScheduler(Duration),
}

/// Supported model worker backends.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ModelRunner {
    /// External model worker reached over ZeroMQ.
    Zmq(u64),
    /// In-process ONNX model loaded from a path or JSON config.
    Onnx(String),
}
