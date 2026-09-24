use futures::future::pending;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::arch::{
    infra_core::env_core::EnvCore,
    strategy_base::{
        command::command_core::{CommandHandle, CommandRegistry, TaskCommand},
        hlist_core::HNil,
    },
    task_execution::{
        TaskInfo, alt_runner::AltTaskRunner, task_alt::AltTaskInfo, task_ws::WsTaskInfo,
        ws_runner::WsTaskRunner,
    },
    traits::{market_lob::WsDecoders, strategy::Strategy},
};

/// Executable runtime produced by [`EnvBuilder`].
///
/// The mediator initializes strategies, registers tasks, provides the command
/// registry to strategies, spawns strategy event loops, and then keeps the
/// runtime alive.
///
/// [`EnvBuilder`]: crate::arch::infra_core::env_builder::EnvBuilder
pub struct EnvMediator<S, D = HNil> {
    pub(crate) core: EnvCore<S>,
    pub(crate) tasks: Vec<TaskInfo>,
    pub(crate) ws_decoders: D,
}

impl<S, D> EnvMediator<S, D>
where
    S: Strategy,
    D: WsDecoders,
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

    fn prepare_tasks(&self) -> (Vec<Arc<CommandHandle>>, Vec<PreparedTask<D>>) {
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
    ) -> Vec<(Arc<CommandHandle>, PreparedTask<D>)> {
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

                let ws_task = WsTaskRunner {
                    cmd_rx,
                    event_tx,
                    ws_info: ws_task_info.clone(),
                    task_id,
                };

                (handle, PreparedTask::Ws(ws_task, self.ws_decoders.clone()))
            })
            .collect()
    }

    fn prepare_alt_tasks(
        &self,
        alt_task_info: &Arc<AltTaskInfo>,
        task_ids: impl IntoIterator<Item = u64>,
    ) -> Vec<(Arc<CommandHandle>, PreparedTask<D>)> {
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

                let alt_task = AltTaskRunner {
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

enum PreparedTask<D> {
    Ws(WsTaskRunner, D),
    Alt(AltTaskRunner),
}

impl<D: WsDecoders> PreparedTask<D> {
    fn spawn(self) {
        match self {
            Self::Ws(mut task, decoders) => {
                tokio::spawn(async move { task.ws_mid_relay(decoders).await });
            },
            Self::Alt(mut task) => {
                tokio::spawn(async move { task.alt_mid_relay().await });
            },
        }
    }
}
