use std::sync::Arc;
use std::future::ready;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::info;

use crate::strategy_base::{
    command::command_core::CommandHandle,
    handler::{
        handler_core::*,
        alt_events::*,
        cex_events::*,
    }
};
use crate::task_execution::task_general::TaskInfo;
use crate::traits::strategy::*;

macro_rules! hlist {
    () => {
        HNil
    };
    ($head:expr $(, $tail:expr)*) => {
        HCons {
            head: $head,
            tail: hlist!($($tail),*)
        }
    };
}

#[derive(Clone)]
pub struct HNil;

#[derive(Clone)]
pub struct HCons<Head, Tail> {
    pub head: Head,
    pub tail: Tail,
}


impl Strategy for HNil { async fn execute(&mut self) {} }
impl EventHandler for HNil {}
impl CexEventHandler for HNil {}
impl AltEventHandler for HNil {}
impl CommandEmitter for HNil {
    fn command_init(&mut self, _command_handle: CommandHandle) {}
}


impl<Head, Tail> Strategy for HCons<Head, Tail>
where
    Head: Strategy,
    Tail: Strategy,
{
    fn execute(&mut self) -> impl Future<Output=()> + Send {
        let fut_head = self.head.execute();
        let fut_tail = self.tail.execute();
        async move { fut_head.await; fut_tail.await; }
    }

    async fn spawn_strategy_tasks(&self, channels: Arc<Vec<BoardCastChannel>>) {
        let HCons { head, tail } = self;
        let ch = channels.clone();
        let h = head.clone();
        info!("Try spawn task for {}", h.name());

        tokio::spawn(async move {
            info!("Spawn task for {}", h.name());
            strategy_handler_loop(h, ch).await;
        });

        tail.spawn_strategy_tasks(channels).await;
    }
}

impl<Head, Tail> EventHandler for HCons<Head, Tail>
where
    Head: Strategy,
    Tail: Strategy,
{
    fn event_init(&mut self, task_info: Arc<TaskInfo>) -> impl Future<Output=()> + Send {
        let fut_head = self.head.event_init(task_info.clone());
        let fut_tail = self.tail.event_init(task_info);
        async move { fut_head.await; fut_tail.await; }
    }
}


impl<Head, Tail> AltEventHandler for HCons<Head, Tail>
where
    Head: AltEventHandler,
    Tail: AltEventHandler,
{
    fn on_timer(&mut self) -> impl Future<Output=()> + Send {
        let fut_head = self.head.on_timer();
        let fut_tail = self.tail.on_timer();
        async move { fut_head.await; fut_tail.await; }
    }
}

impl<Head, Tail> CexEventHandler for HCons<Head, Tail>
where
    Head: CexEventHandler,
    Tail: CexEventHandler,
{
    fn on_candle(&mut self, msg: Arc<Vec<WsCandle>>) -> impl Future<Output=()> + Send {
        let fut_head = self.head.on_candle(msg.clone());
        let fut_tail = self.tail.on_candle(msg);
        async move { fut_head.await; fut_tail.await; }
    }
    fn on_trade(&mut self, msg: Arc<Vec<WsTrade>>) -> impl Future<Output=()> + Send {
        let fut_head = self.head.on_trade(msg.clone());
        let fut_tail = self.tail.on_trade(msg);
        async move { fut_head.await; fut_tail.await; }
    }

    fn on_lob(&mut self, msg: Arc<Vec<WsLob>>) -> impl Future<Output=()> + Send {
        let fut_head = self.head.on_lob(msg.clone());
        let fut_tail = self.tail.on_lob(msg);
        async move { fut_head.await; fut_tail.await; }
    }
}

impl<Head, Tail> CommandEmitter for HCons<Head, Tail>
where
    Head: CommandEmitter,
    Tail: CommandEmitter,
{
    fn command_init(&mut self, command_handle: CommandHandle) {
        self.head.command_init(command_handle.clone());
        self.tail.command_init(command_handle);
    }
}


