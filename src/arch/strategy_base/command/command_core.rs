use tokio::sync::{
    oneshot,
    mpsc,
};

use crate::errors::{InfraError, InfraResult};
use crate::arch::{
    market_assets::api_general::OrderParams,
    strategy_base::{
        command::ack_handle::{AckHandle, AckStatus},
        handler::alt_events::AltTensor,
    },
    task_execution::task_general::TaskInfo,
};

#[derive(Clone, Debug)]
pub struct CommandHandle {
    pub cmd_tx: mpsc::Sender<TaskCommand>,
    pub task_info: TaskInfo,
    pub task_id: u64,
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
            .map_err(|e| InfraError::Msg(format!("Failed to send Command: {}", e)))?;

        if let Some((expected, rx)) = expected_ack {
            let ack = rx.await.map_err(|_| InfraError::Msg("Ack channel closed".into()))?;
            if ack == expected {
                Ok(())
            } else {
                Err(InfraError::Msg(format!(
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
    WsConnect { msg: String, ack: AckHandle },
    WsMessage { msg: String, ack: AckHandle },
    WsShutdown { msg: String, ack: AckHandle },

    OrderExecute(Vec<OrderParams>),
    FeatInput(AltTensor),
}

impl TaskCommand {
    pub fn get_ack(self) -> Option<AckHandle> {
        match self {
            TaskCommand::WsMessage { ack, .. }
            | TaskCommand::WsShutdown { ack, .. } => Some(ack),
            _ => None,
        }
    }
}



