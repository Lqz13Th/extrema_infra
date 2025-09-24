use std::future::ready;
use std::sync::Arc;
use crate::market_assets::api_general::OrderParams;
use crate::strategy_base::{
    command::command_core::CommandHandle,
    handler::{
        handler_core::{BoardCastChannel, InfraMsg},
        alt_events::*,
        cex_events::*,
    }
};
use crate::task_execution::{
    task_general::TaskInfo,
    task_alt::{AltTaskInfo, AltTaskType},
    task_ws::{WsTaskInfo, WsChannel},
};

pub trait Strategy: EventHandler + CommandEmitter {
    fn initialize(&mut self) -> impl Future<Output=()> + Send;
    fn strategy_name(&self) -> &'static str { std::any::type_name::<Self>() }
    fn _spawn_strategy_tasks(
        &self,
        _channels: &Arc<Vec<BoardCastChannel>>
    )  -> impl Future<Output=()> + Send { ready(()) }
}

pub trait EventHandler: AltEventHandler + CexEventHandler + DexEventHandler {}

pub trait AltEventHandler: Clone + Send + Sync + 'static {
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
}

pub trait CexEventHandler: Clone + Send + Sync + 'static {
    fn on_cex_event(
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

pub trait DexEventHandler: Clone + Send + Sync + 'static {
    fn on_dex_event(
        &mut self,
        _msg: InfraMsg<WsTaskInfo>
    ) -> impl Future<Output=()> + Send { ready(()) }
}

pub trait CommandEmitter: Clone + Send + Sync + 'static {
    fn command_init(&mut self, _command_handle: Arc<CommandHandle>);
    fn command_registry(&self) -> Vec<Arc<CommandHandle>>;

    fn find_alt_handle(
        &self,
        alt_task_type: &AltTaskType,
        task_numb: u64
    ) -> Option<Arc<CommandHandle>> {
        self.command_registry().iter().find_map(|handle| {
            match &handle.task_info {
                TaskInfo::AltTask(task)
                if &task.alt_task_type == alt_task_type && handle.task_numb == task_numb
                => Some(handle.clone()),
                _ => None,
            }
        })
    }

    fn find_ws_handle(
        &self,
        channel: &WsChannel,
        task_numb: u64
    ) -> Option<Arc<CommandHandle>> {
        self.command_registry().iter().find_map(|handle| {
            match &handle.task_info {
                TaskInfo::WsTask(task)
                if task.ws_channel == *channel && handle.task_numb == task_numb
                => Some(handle.clone()),
                _ => None,
            }
        })
    }
}
