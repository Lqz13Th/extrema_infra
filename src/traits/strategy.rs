use std::future::ready;
use std::sync::Arc;
use crate::strategy_base::{
    command::command_core::CommandHandle,
    handler::{
        handler_core::BoardCastChannel,
        alt_events::*,
        cex_events::*,
    }
};
use crate::task_execution::{
    task_alt::AltTaskInfo,
    task_ws::WsTaskInfo,
};

pub trait Strategy: EventHandler + CommandEmitter {
    fn execute(&mut self) -> impl Future<Output=()> + Send;
    fn strategy_name(&self) -> &'static str { std::any::type_name::<Self>() }
    fn spawn_strategy_tasks(
        &self,
        _channels: &Arc<Vec<BoardCastChannel>>
    )  -> impl Future<Output=()> + Send { ready(()) }
}

pub trait EventHandler: AltEventHandler + CexEventHandler + DexEventHandler {}

pub trait AltEventHandler: Clone + Send + Sync + 'static {
    fn on_alt_event(
        &mut self,
        _msg: Arc<AltTaskInfo>
    ) -> impl Future<Output=()> + Send { ready(()) }

    fn on_timer(
        &mut self, 
        _msg: Arc<AltTimerEvent>
    ) -> impl Future<Output=()> + Send { ready(()) }
}

pub trait CexEventHandler: Clone + Send + Sync + 'static {
    fn on_cex_event(
        &mut self,
        _msg: Arc<WsTaskInfo>
    ) -> impl Future<Output=()> + Send { ready(()) }

    fn on_trade(&mut self, _msg: Arc<Vec<WsTrade>>) -> impl Future<Output=()> + Send { ready(()) }
    fn on_lob(&mut self, _msg: Arc<Vec<WsLob>>) -> impl Future<Output=()> + Send { ready(()) }
    fn on_candle(&mut self, _msg: Arc<Vec<WsCandle>>) -> impl Future<Output=()> + Send { ready(()) }
    fn on_account_order(
        &mut self, 
        _msg: Arc<Vec<WsLob>>
    ) -> impl Future<Output=()> + Send { ready(()) }

}

pub trait DexEventHandler: Clone + Send + Sync + 'static {
    fn on_dex_event(
        &mut self,
        _msg: Arc<WsTaskInfo>
    ) -> impl Future<Output=()> + Send { ready(()) }
}




pub trait CommandEmitter: Clone + Send + Sync + 'static {
    fn command_init(&mut self, _command_handle: Arc<CommandHandle>);
}
