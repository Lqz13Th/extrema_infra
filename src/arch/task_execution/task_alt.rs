use std::time::Duration;

#[derive(Clone, Debug)]
pub struct AltTaskInfo {
    pub alt_task_type: AltTaskType,
    pub chunk: u64,
    pub task_base_id: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AltTaskType {
    OrderExecution,
    InstIntent,
    ModelPreds(u64),
    TimeScheduler(Duration),
}
