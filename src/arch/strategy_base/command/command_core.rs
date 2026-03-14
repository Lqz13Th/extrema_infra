use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, oneshot};

use crate::arch::{
    strategy_base::{
        command::ack_handle::{AckHandle, AckStatus},
        handler::alt_events::{AltIntent, AltOrder, AltTensor},
    },
    task_execution::{task_alt::AltTaskType, task_general::TaskInfo, task_ws::WsChannel},
};
use crate::errors::{InfraError, InfraResult};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CommandKey {
    Alt {
        alt_task_type: AltTaskType,
        task_id: u64,
    },
    Ws {
        ws_channel: WsChannel,
        task_id: u64,
    },
}

#[derive(Clone, Debug, Default)]
pub struct CommandRegistry {
    handles: Arc<HashMap<CommandKey, Arc<CommandHandle>>>,
}

impl CommandRegistry {
    pub fn new(handles: Vec<Arc<CommandHandle>>) -> Self {
        let mut map = HashMap::with_capacity(handles.len());

        for handle in handles {
            let key = match &handle.task_info {
                TaskInfo::AltTask(task) => CommandKey::Alt {
                    alt_task_type: task.alt_task_type.clone(),
                    task_id: handle.task_id,
                },
                TaskInfo::WsTask(task) => CommandKey::Ws {
                    ws_channel: task.ws_channel.clone(),
                    task_id: handle.task_id,
                },
            };

            if let Some(old) = map.insert(key.clone(), handle.clone()) {
                panic!(
                    "Duplicate CommandKey in registry: {:?}, old={:?}, new={:?}",
                    key, old, handle
                );
            }
        }

        Self {
            handles: Arc::new(map),
        }
    }

    pub fn find_alt_handle(
        &self,
        alt_task_type: &AltTaskType,
        task_id: u64,
    ) -> Option<Arc<CommandHandle>> {
        self.handles
            .get(&CommandKey::Alt {
                alt_task_type: alt_task_type.clone(),
                task_id,
            })
            .cloned()
    }

    pub fn find_ws_handle(
        &self,
        ws_channel: &WsChannel,
        task_id: u64,
    ) -> Option<Arc<CommandHandle>> {
        self.handles
            .get(&CommandKey::Ws {
                ws_channel: ws_channel.clone(),
                task_id,
            })
            .cloned()
    }
}

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
            let ack = rx.await.map_err(|_| {
                InfraError::Msg(format!("Ack channel closed, expected ack: {:?}", expected,))
            })?;

            if ack == expected {
                Ok(())
            } else {
                Err(InfraError::Msg(format!(
                    "Unexpected ack: {:?}, expected: {:?}",
                    ack, expected,
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

    OrderExecute(Vec<AltOrder>),
    InstIntent(AltIntent),
    FeatInput(AltTensor),
}

impl TaskCommand {
    pub fn get_ack(self) -> Option<AckHandle> {
        match self {
            TaskCommand::WsMessage { ack, .. } | TaskCommand::WsShutdown { ack, .. } => Some(ack),
            _ => None,
        }
    }
}
