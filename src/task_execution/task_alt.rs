#[derive(Clone, Debug)]
pub struct AltTaskInfo {
    pub alt_task_type: AltTaskType,
}

#[derive(Clone, Debug)]
pub enum AltTaskType {
    NeuralNetwork(u64),
    TimerBasedState(u64),
}