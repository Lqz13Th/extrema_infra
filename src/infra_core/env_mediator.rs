use tracing::info;

use crate::traits::strategy::Strategy;
use crate::infra_core::env_core::EnvCore;
use crate::task_execution::{
    general_register::TaskInfo,
    alt_register::*,
    ws_register::*,
};

pub struct EnvMediator<S> {
    pub(crate) core: EnvCore<S>,
    pub(crate) tasks: Vec<TaskInfo>,
}

impl<S> EnvMediator<S>
where S: Strategy + Clone
{
    pub async fn initialize(
        mut self,
    ) -> Self {
        todo!()
    }

    pub async fn execute(self) {
        self.spawn_strategies().await;
        self.register_tasks();
        futures::future::pending::<()>().await;
    }

    async fn spawn_strategies(&self) {
        let strategies = self.core.strategies.clone();
        let channels = self.core.board_cast_channels.clone();
        info!("spawn_strategy");
        strategies.spawn_strategy_tasks(channels.clone()).await;
    }

    fn register_tasks(&self) {
        for env_task in &self.tasks {
            match env_task {
                TaskInfo::WsTask(ws_task_info) => {
                    self.spawn_ws_tasks(ws_task_info);
                },
                TaskInfo::AltTask(alt_task_info) => {
                    self.spawn_alt_tasks(alt_task_info);
                },
            };
        }
    }

    fn spawn_ws_tasks(
        &self,
        ws_task_info: &WsTaskInfo,
    ) {
        for chunk_numb in 0..ws_task_info.chunk {
            let core_clone = self.core.clone();

            let mut ws_task = WsTaskBuilder {
                core: core_clone,
                ws_info: ws_task_info.clone(),
                task_numb: chunk_numb,
            };

            tokio::spawn(async move {
                ws_task.ws_mid_relay().await
            });
        }
    }

    fn spawn_alt_tasks(
        &self,
        alt_task_info: &AltTaskInfo,
    ) {
        let core_clone = self.core.clone();
        let mut alt_task = AltTaskBuilder {
            core: core_clone,
            alt_info: alt_task_info.clone(),
        };

        tokio::spawn(async move {
            alt_task.alt_mid_relay().await
        });
    }
}



