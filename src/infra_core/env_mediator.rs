use std::sync::Arc;
use tokio::sync::mpsc;

use crate::infra_core::env_core::EnvCore;
use crate::strategy_base::command::command_core::{CommandHandle, TaskCommand};
use crate::task_execution::{
    task_general::TaskInfo,
    task_alt::AltTaskInfo,
    task_ws::WsTaskInfo,
    register_alt::AltTaskBuilder,
    register_ws::WsTaskBuilder,

};
use crate::traits::strategy::Strategy;

pub struct EnvMediator<S> {
    pub(crate) core: EnvCore<S>,
    pub tasks: Vec<TaskInfo>,
}

impl<S> EnvMediator<S>
where
    S: Strategy,
{
    pub async fn execute(mut self) {
        let handles = self.register_tasks();
        println!("handles: {:?}", handles);
        for handle in &handles {
            self.core.strategy.command_init(handle.clone());
        }

        self.core.strategy.spawn_strategy_tasks(&self.core.channel).await;
        self.core.strategy.execute().await;
        futures::future::pending::<()>().await;
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
        (0..ws_task_info.chunk)
            .map(|chunk_numb| {
                let (cmd_tx, cmd_rx) = mpsc::channel::<TaskCommand>(2048);
                let handle = Arc::new(CommandHandle {
                    task_info: TaskInfo::WsTask(ws_task_info.clone()),
                    cmd_tx,
                });

                let mut ws_task = WsTaskBuilder {
                    cmd_rx,
                    board_cast_channel: self.core.channel.clone(),
                    ws_info: ws_task_info.clone(),
                    task_numb: chunk_numb as u64,
                };

                tokio::spawn(async move { ws_task.ws_mid_relay().await });

                handle
            })
            .collect()
    }

    fn spawn_alt_tasks(&self, alt_task_info: &Arc<AltTaskInfo>) -> Vec<Arc<CommandHandle>> {
        let (cmd_tx, cmd_rx) = mpsc::channel::<TaskCommand>(2048);
        let handle = Arc::new(CommandHandle {
            task_info: TaskInfo::AltTask(alt_task_info.clone()),
            cmd_tx,
        });

        let mut alt_task = AltTaskBuilder {
            cmd_rx,
            board_cast_channel: self.core.channel.clone(),
            alt_info: alt_task_info.clone(),
        };

        tokio::spawn(async move { alt_task.alt_mid_relay().await });

        vec![handle]
    }
}

