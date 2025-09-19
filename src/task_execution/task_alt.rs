#[derive(Clone, Debug)]
pub struct AltTaskInfo {
    pub alt_task_type: AltTaskType,
    pub chunk: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AltTaskType {
    NeuralNetwork(u64),
    TimerBasedState(u64),
}