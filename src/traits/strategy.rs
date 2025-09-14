use std::future::ready;
use std::sync::Arc;

use crate::strategy_base::{
    command::command_core::CommandHandle,
    handler::{
        handler_core::BoardCastChannel,
        cex_events::*,
        alt_events::*,
    }
};
use crate::task_execution::task_general::TaskInfo;

pub trait Strategy: EventHandler + CommandEmitter {
    fn execute(&mut self) -> impl Future<Output=()> + Send;

    fn name(&self) -> &'static str { "unnamed_strategy" }
    fn spawn_strategy_tasks(
        &self,
        _channels: Arc<Vec<BoardCastChannel>>
    )  -> impl Future<Output=()> + Send { ready(()) }

}

pub trait EventHandler: CexEventHandler + AltEventHandler {
    fn event_init(
        &mut self,
        _task_info: Arc<TaskInfo>,
    ) -> impl Future<Output=()> + Send { ready(()) }
}

pub trait CexEventHandler: Clone + Send + Sync + 'static {
    fn on_candle(&mut self, _msg: Arc<Vec<WsCandle>>) -> impl Future<Output=()> + Send { ready(()) }

    fn on_trade(&mut self, _msg: Arc<Vec<WsTrade>>) -> impl Future<Output=()> + Send { ready(()) }
    fn on_lob(&mut self, _msg: Arc<Vec<WsLob>>) -> impl Future<Output=()> + Send { ready(()) }
}

pub trait AltEventHandler: Clone + Send + Sync + 'static {
    fn on_timer(&mut self) -> impl Future<Output=()> + Send { ready(()) }

}


pub trait CommandEmitter: Clone + Send + Sync + 'static {

    fn command_init(&mut self, _command_handle: CommandHandle);
}
