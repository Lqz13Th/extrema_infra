use std::sync::Arc;
use tokio::sync::{
    oneshot,
    mpsc,
};

use crate::errors::{InfraError, InfraResult};
use crate::strategy_base::command::ack_handle::{AckHandle, AckStatus};
use crate::task_execution::task_general::TaskInfo;

#[derive(Clone, Debug)]
pub struct CommandHandle {
    pub cmd_tx: mpsc::Sender<TaskCommand>,
    pub task_info: TaskInfo,
    pub task_numb: u64,
}

impl CommandHandle {
    pub async fn send_command(
        &self,
        cmd: TaskCommand,
        expected_ack: Option<(AckStatus, oneshot::Receiver<AckStatus>)>,
    ) -> InfraResult<()> {
        self.cmd_tx
            .send(cmd)
            .await
            .map_err(|e| InfraError::Other(format!("Failed to send Command: {}", e)))?;

        if let Some((expected, rx)) = expected_ack {
            let ack = rx.await.map_err(|_| InfraError::Other("Ack channel closed".into()))?;
            if ack == expected {
                Ok(())
            } else {
                Err(InfraError::Other(format!(
                    "Unexpected ack: {:?}, expected: {:?}",
                    ack, expected
                )))
            }
        } else {
            Ok(())
        }
    }

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
    pub fn get_ack(self) -> Option<AckHandle> {
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
