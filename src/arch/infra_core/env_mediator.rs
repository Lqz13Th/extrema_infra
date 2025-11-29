use std::sync::Arc;
use tokio::sync::mpsc;
use futures::future::pending;

use crate::arch::{
    infra_core::env_core::EnvCore,
    strategy_base::command::command_core::{CommandHandle, TaskCommand},
    task_execution::{
        task_general::TaskInfo,
        task_alt::AltTaskInfo,
        task_ws::WsTaskInfo,
        register_alt::AltTaskBuilder,
        register_ws::WsTaskBuilder,
    },
    traits::strategy::Strategy,
};

pub struct EnvMediator<S> {
    pub(crate) core: EnvCore<S>,
    pub tasks: Vec<TaskInfo>,
}

impl<S> EnvMediator<S>
where
    S: Strategy,
{
    pub async fn execute(mut self) {
        self.core.strategy.initialize().await;
        let handles = self.register_tasks();
        for handle in &handles {
            self.core.strategy.command_init(handle.clone());
        }

        self.core.strategy._spawn_strategy_tasks(&self.core.channel).await;
        pending::<()>().await;
    }

    fn register_tasks(&self) -> Vec<Arc<CommandHandle>> {
        self.tasks
            .iter()
            .flat_map(|task| match task {
                TaskInfo::WsTask(ws) => self.spawn_ws_tasks(ws),
                TaskInfo::AltTask(alt) => self.spawn_alt_tasks(alt),
            })
            .collect()
    }

    fn spawn_ws_tasks(&self, ws_task_info: &Arc<WsTaskInfo>) -> Vec<Arc<CommandHandle>> {
        (1..=ws_task_info.chunk)
            .map(|chunk_numb| {
                let task_id = match ws_task_info.task_base_id {
                    Some(base) => base + chunk_numb - 1,
                    None => chunk_numb,
                };

                let (cmd_tx, cmd_rx) = mpsc::channel::<TaskCommand>(2048);
                let handle = Arc::new(CommandHandle {
                    cmd_tx,
                    task_info: TaskInfo::WsTask(ws_task_info.clone()),
                    task_id,
                });

                let mut ws_task = WsTaskBuilder {
                    cmd_rx,
                    board_cast_channel: self.core.channel.clone(),
                    ws_info: ws_task_info.clone(),
                    filter_channels: ws_task_info.filter_channels,
                    task_id,
                };

                tokio::spawn(async move { ws_task.ws_mid_relay().await });

                handle
            })
            .collect()
    }

    fn spawn_alt_tasks(&self, alt_task_info: &Arc<AltTaskInfo>) -> Vec<Arc<CommandHandle>> {
        (1..=alt_task_info.chunk)
            .map(|chunk_numb| {
                let task_id = match alt_task_info.task_base_id {
                    Some(base) => base + chunk_numb - 1,
                    None => chunk_numb,
                };

                let (cmd_tx, cmd_rx) = mpsc::channel::<TaskCommand>(2048);
                let handle = Arc::new(CommandHandle {
                    cmd_tx,
                    task_info: TaskInfo::AltTask(alt_task_info.clone()),
                    task_id,
                });

                let mut alt_task = AltTaskBuilder {
                    cmd_rx,
                    board_cast_channel: self.core.channel.clone(),
                    alt_info: alt_task_info.clone(),
                    task_id,
                };

                tokio::spawn(async move { alt_task.alt_mid_relay().await });

                handle
            })
            .collect()
    }
}

