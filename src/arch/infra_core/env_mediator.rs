use futures::future::pending;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::arch::{
    infra_core::env_core::EnvCore,
    strategy_base::command::command_core::{CommandHandle, CommandRegistry, TaskCommand},
    task_execution::{
        register_alt::AltTaskBuilder, register_ws::WsTaskBuilder, task_alt::AltTaskInfo,
        task_general::TaskInfo, task_ws::WsTaskInfo,
    },
    traits::strategy::Strategy,
};

/// Executable runtime produced by [`EnvBuilder`].
///
/// The mediator initializes strategies, registers tasks, provides the command
/// registry to strategies, spawns strategy event loops, and then keeps the
/// runtime alive.
///
/// [`EnvBuilder`]: crate::arch::infra_core::env_builder::EnvBuilder
pub struct EnvMediator<S> {
    pub(crate) core: EnvCore<S>,
    pub(crate) tasks: Vec<TaskInfo>,
}

impl<S> EnvMediator<S>
where
    S: Strategy,
{
    /// Returns the runtime task declarations captured when the environment was built.
    pub fn tasks(&self) -> &[TaskInfo] {
        &self.tasks
    }

    /// Starts the environment and waits forever.
    ///
    /// This method is intended to be the last awaited call in a strategy binary.
    pub async fn execute(mut self) {
        self.core.strategy.initialize().await;

        let (command_handles, prepared_tasks) = self.prepare_tasks();
        let command_registry = Arc::new(CommandRegistry::new(command_handles));

        self.core
            .strategy
            .command_init(Arc::clone(&command_registry));

        self.core
            .strategy
            ._spawn_strategy_tasks(&self.core.task_channels)
            .await;

        for task in prepared_tasks {
            task.spawn();
        }

        pending::<()>().await;
    }

    fn prepare_tasks(&self) -> (Vec<Arc<CommandHandle>>, Vec<PreparedTask>) {
        let mut handles = Vec::new();
        let mut tasks = Vec::new();

        for task in &self.tasks {
            let task_ids = task
                .task_ids()
                .expect("EnvBuilder validated every task-id range");
            let prepared = match task {
                TaskInfo::WsTask(ws) => self.prepare_ws_tasks(ws, task_ids),
                TaskInfo::AltTask(alt) => self.prepare_alt_tasks(alt, task_ids),
            };
            for (handle, task) in prepared {
                handles.push(handle);
                tasks.push(task);
            }
        }

        (handles, tasks)
    }

    fn prepare_ws_tasks(
        &self,
        ws_task_info: &Arc<WsTaskInfo>,
        task_ids: impl IntoIterator<Item = u64>,
    ) -> Vec<(Arc<CommandHandle>, PreparedTask)> {
        task_ids
            .into_iter()
            .map(|task_id| {
                let (cmd_tx, cmd_rx) = mpsc::channel::<TaskCommand>(2048);
                let handle = Arc::new(CommandHandle {
                    cmd_tx,
                    task_info: TaskInfo::WsTask(ws_task_info.clone()),
                    task_id,
                });
                let task_key = handle.task_info.task_key(task_id);
                let event_tx = self
                    .core
                    .task_channels
                    .sender(&task_key)
                    .expect("EnvBuilder created a channel for every concrete task");

                let ws_task = WsTaskBuilder {
                    cmd_rx,
                    event_tx,
                    ws_info: ws_task_info.clone(),
                    filter_channels: ws_task_info.filter_channels,
                    task_id,
                };

                (handle, PreparedTask::Ws(ws_task))
            })
            .collect()
    }

    fn prepare_alt_tasks(
        &self,
        alt_task_info: &Arc<AltTaskInfo>,
        task_ids: impl IntoIterator<Item = u64>,
    ) -> Vec<(Arc<CommandHandle>, PreparedTask)> {
        task_ids
            .into_iter()
            .map(|task_id| {
                let (cmd_tx, cmd_rx) = mpsc::channel::<TaskCommand>(2048);
                let handle = Arc::new(CommandHandle {
                    cmd_tx,
                    task_info: TaskInfo::AltTask(alt_task_info.clone()),
                    task_id,
                });
                let task_key = handle.task_info.task_key(task_id);
                let event_tx = self
                    .core
                    .task_channels
                    .sender(&task_key)
                    .expect("EnvBuilder created a channel for every concrete task");

                let alt_task = AltTaskBuilder {
                    cmd_rx,
                    event_tx,
                    alt_info: alt_task_info.clone(),
                    task_id,
                };

                (handle, PreparedTask::Alt(alt_task))
            })
            .collect()
    }
}

enum PreparedTask {
    Ws(WsTaskBuilder),
    Alt(AltTaskBuilder),
}

impl PreparedTask {
    fn spawn(self) {
        match self {
            Self::Ws(mut task) => {
                tokio::spawn(async move { task.ws_mid_relay().await });
            },
            Self::Alt(mut task) => {
                tokio::spawn(async move { task.alt_mid_relay().await });
            },
        }
    }
}
