use std::sync::Arc;
use tokio::sync::mpsc;

use crate::strategy_base::command::ack_handle::AckHandle;
use crate::task_execution::task_general::TaskInfo;

#[derive(Clone, Debug)]
pub struct CommandHandle {
    pub task_info: TaskInfo,
    pub cmd_tx: mpsc::Sender<TaskCommand>,
}

#[derive(Debug)]
pub enum TaskCommand {
    Connect { msg: String, ack: AckHandle },
    Subscribe { msg: String, ack: AckHandle },
    Unsubscribe { msg: String, ack: AckHandle },
    Shutdown { msg: String, ack: AckHandle },
    Login { msg: String, ack: AckHandle }, 

    NNInput(Arc<NeuralInput>),
    NNOutput(Arc<NeuralOutput>),
}

impl TaskCommand {
    pub fn ack(self) -> Option<AckHandle> {
        match self {
            TaskCommand::Connect { ack, .. }
            | TaskCommand::Subscribe { ack, .. }
            | TaskCommand::Unsubscribe { ack, .. }
            | TaskCommand::Shutdown { ack, .. } => Some(ack),
            _ => None,
        }
    }
}


#[derive(Clone, Debug)]
pub struct NeuralInput {
    pub features: Vec<f32>,
    pub n_rows: usize,
    pub n_cols: usize,
    pub timestamp: i64,
}

#[derive(Clone, Debug)]
pub struct NeuralOutput {
    pub probs: Vec<f32>,
    pub n_rows: usize,
    pub n_cols: usize,
    pub value: f32,
}
