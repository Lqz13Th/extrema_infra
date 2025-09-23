use std::time::Duration;

#[derive(Clone, Debug)]
pub struct AltTaskInfo {
    pub alt_task_type: AltTaskType,
    pub chunk: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AltTaskType {
    OrderExecution(),
    TimeScheduler(Duration),
    NeuralNetwork(u64),
}