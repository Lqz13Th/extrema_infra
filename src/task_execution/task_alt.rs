#[derive(Debug, Clone)]
pub struct AltTaskInfo {
    pub alt_task_type: AltTaskType,
}

#[derive(Debug, Clone)]
pub enum AltTaskType {
    NeuralNetwork(u64),
    TimerBasedState(u64),
}