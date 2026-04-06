use std::time::Duration;

#[derive(Clone, Debug)]
pub struct AltTaskInfo {
    pub alt_task_type: AltTaskType,
    pub chunk: u64,
    pub task_base_id: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AltTaskType {
    OrderExecution,
    InstIntent,
    ModelPreds(ModelRunner),
    TimeScheduler(Duration),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ModelRunner {
    Zmq(u64),
    Onnx(String),
}
