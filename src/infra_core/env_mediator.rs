use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

use crate::traits::strategy::Strategy;
use crate::infra_core::env_core::EnvCore;
use crate::strategy_base::command::command_core::{CommandHandle, TaskCommand};
use crate::task_execution::{
    task_general::TaskInfo,
    alt_register::*,
    ws_register::*,
};

pub struct EnvMediator<S> {
    pub(crate) core: EnvCore<S>,
    pub(crate) tasks: Vec<TaskInfo>,
}

impl<S> EnvMediator<S>
where
    S: Strategy + Clone,
{
    pub async fn initialize(mut self) -> Self {
        info!("Spawning strategy tasks...");
        self.core
            .strategy
            .spawn_strategy_tasks(self.core.channel.clone())
            .await;

        info!("Registering board cast tasks...");
        self.register_tasks()
            .into_iter()
            .for_each(|handle| self.core.strategy.command_init(handle));

        self
    }

    pub async fn execute(&mut self) {
        self.core.strategy.execute().await;
        futures::future::pending::<()>().await;
    }

    fn register_tasks(&self) -> Vec<CommandHandle> {
        self.tasks
            .iter()
            .flat_map(|task| match task {
                TaskInfo::WsTask(ws) => self.spawn_ws_tasks(ws),
                TaskInfo::AltTask(alt) => self.spawn_alt_tasks(alt),
            })
            .collect()
    }

    fn spawn_ws_tasks(&self, ws_task_info: &Arc<WsTaskInfo>) -> Vec<CommandHandle> {
        (0..ws_task_info.chunk)
            .map(|chunk_numb| {
                let (cmd_tx, cmd_rx) = mpsc::channel::<TaskCommand>(100);
                let handle = CommandHandle {
                    task_info: TaskInfo::WsTask(ws_task_info.clone()),
                    cmd_tx,
                };

                let mut ws_task = WsTaskBuilder {
                    cmd_rx,
                    channel: self.core.channel.clone(),
                    ws_info: ws_task_info.clone(),
                    task_numb: chunk_numb,
                };

                tokio::spawn(async move { ws_task.ws_mid_relay().await });

                handle
            })
            .collect()
    }

    fn spawn_alt_tasks(&self, alt_task_info: &Arc<AltTaskInfo>) -> Vec<CommandHandle> {
        let (cmd_tx, cmd_rx) = mpsc::channel::<TaskCommand>(100);
        let handle = CommandHandle {
            task_info: TaskInfo::AltTask(alt_task_info.clone()),
            cmd_tx,
        };

        let mut alt_task = AltTaskBuilder {
            cmd_rx,
            channel: self.core.channel.clone(),
            alt_info: alt_task_info.clone(),
        };

        tokio::spawn(async move { alt_task.alt_mid_relay().await });

        vec![handle]
    }
}

