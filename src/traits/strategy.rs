use std::future::ready;
use std::sync::Arc;

use crate::task_execution::ws_register::{SharedWsWrite, WsTaskHandle};
use crate::strategy_base::event_notify::{
    cex_notify::*,
    alt_notify::*,
};
use crate::strategy_base::event_notify::board_cast_channels::BoardCastChannel;

pub trait Strategy: AltNotify + CexNotify + TaskOperation {
    fn name(&self) -> &'static str { "unnamed_strategy" }
    fn spawn_strategy_tasks(
        &self,
        _channels: Arc<Vec<BoardCastChannel>>
    )  -> impl Future<Output=()> + Send { ready(()) }

}

pub trait AltNotify: Clone + Send + Sync + 'static {
    fn on_timer(&mut self) -> impl Future<Output=()> + Send { ready(()) }
}

pub trait CexNotify: Clone + Send + Sync + 'static {
    fn on_trade(&mut self, _msg: Arc<Vec<WsTrade>>) -> impl Future<Output=()> + Send { ready(()) }
    fn on_lob(&mut self, _msg: Arc<Vec<WsLob>>) -> impl Future<Output=()> + Send { ready(()) }
}

pub trait TaskOperation: Clone + Send + Sync + 'static {

    fn on_ws_init(
        &mut self,
        _ws_task_handle: WsTaskHandle,
    ) -> impl Future<Output=()> + Send { ready(()) }
}
