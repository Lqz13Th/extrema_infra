use std::{
    sync::Arc,
    future::ready,
};

use crate::arch::{
    market_assets::api_general::OrderParams,
    strategy_base::{
        command::command_core::CommandHandle,
        handler::{
            handler_core::{BoardCastChannel, InfraMsg},
            alt_events::*,
            cex_events::*,
        },
    },
    task_execution::{
        task_general::TaskInfo,
        task_alt::{AltTaskInfo, AltTaskType},
        task_ws::{WsChannel, WsTaskInfo},
    },
};

pub trait Strategy: CommandEmitter + EventHandler {
    fn initialize(&mut self) -> impl Future<Output=()> + Send;
    fn strategy_name(&self) -> &'static str { std::any::type_name::<Self>() }
    fn _spawn_strategy_tasks(
        &self,
        _channels: &Arc<Vec<BoardCastChannel>>
    )  -> impl Future<Output=()> + Send { ready(()) }
}

pub trait CommandEmitter: Clone + Send + Sync + 'static {
    fn command_init(&mut self, _command_handle: Arc<CommandHandle>);
    fn command_registry(&self) -> Vec<Arc<CommandHandle>>;

    fn find_alt_handle(
        &self,
        alt_task_type: &AltTaskType,
        task_id: u64
    ) -> Option<Arc<CommandHandle>> {
        self.command_registry().iter().find_map(|handle| {
            match &handle.task_info {
                TaskInfo::AltTask(task)
                if &task.alt_task_type == alt_task_type && handle.task_id == task_id
                => Some(handle.clone()),
                _ => None,
            }
        })
    }

    fn find_ws_handle(
        &self,
        channel: &WsChannel,
        task_id: u64
    ) -> Option<Arc<CommandHandle>> {
        self.command_registry().iter().find_map(|handle| {
            match &handle.task_info {
                TaskInfo::WsTask(task)
                if task.ws_channel == *channel && handle.task_id == task_id
                => Some(handle.clone()),
                _ => None,
            }
        })
    }
}

pub trait EventHandler {
    // Alt Event
    fn on_alt_event(
        &mut self,
        _msg: InfraMsg<AltTaskInfo>
    ) -> impl Future<Output=()> + Send { ready(()) }

    fn on_order_execution(
        &mut self,
        _msg: InfraMsg<Vec<OrderParams>>
    ) -> impl Future<Output=()> + Send { ready(()) }

    fn on_schedule(
        &mut self,
        _msg: InfraMsg<AltScheduleEvent>
    ) -> impl Future<Output=()> + Send { ready(()) }

    fn on_preds(
        &mut self,
        _msg: InfraMsg<AltMatrix>
    ) -> impl Future<Output=()> + Send { ready(()) }

    // Ws Event
    fn on_ws_event(
        &mut self,
        _msg: InfraMsg<WsTaskInfo>
    ) -> impl Future<Output=()> + Send { ready(()) }

    fn on_trade(
        &mut self,
        _msg: InfraMsg<Vec<WsTrade>>
    ) -> impl Future<Output=()> + Send { ready(()) }

    fn on_lob(
        &mut self,
        _msg: InfraMsg<Vec<WsLob>>
    ) -> impl Future<Output=()> + Send { ready(()) }

    fn on_candle(
        &mut self,
        _msg: InfraMsg<Vec<WsCandle>>
    ) -> impl Future<Output=()> + Send { ready(()) }

    fn on_acc_order(
        &mut self,
        _msg: InfraMsg<Vec<WsAccOrder>>
    ) -> impl Future<Output=()> + Send { ready(()) }
}

