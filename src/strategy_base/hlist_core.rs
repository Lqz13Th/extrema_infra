use std::sync::Arc;
use tracing::info;

use crate::strategy_base::{
    command::command_core::CommandHandle,
    handler::{
        handler_core::*,
        cex_events::*,
    }
};
use crate::task_execution::{
    task_alt::AltTaskInfo,
    task_ws::WsTaskInfo,
};
use crate::traits::strategy::*;

// macro_rules! hlist {
//     () => {
//         HNil
//     };
//     ($head:expr $(, $tail:expr)*) => {
//         HCons {
//             head: $head,
//             tail: hlist!($($tail),*)
//         }
//     };
// }

#[derive(Clone)]
pub struct HNil;

#[derive(Clone)]
pub struct HCons<Head, Tail> {
    pub head: Head,
    pub tail: Tail,
}


impl Strategy for HNil { async fn execute(&mut self) {} }
impl EventHandler for HNil {}
impl AltEventHandler for HNil {}
impl DexEventHandler for HNil {}
impl CexEventHandler for HNil {}
impl CommandEmitter for HNil {
    fn command_init(&mut self, _command_handle: Arc<CommandHandle>) {}
}


impl<Head, Tail> Strategy for HCons<Head, Tail>
where
    Head: Strategy + Send + Sync + Clone + 'static,
    Tail: Strategy + Send + Sync + Clone + 'static
{
    async fn execute(&mut self) {
        let fut_head = self.head.execute();
        let fut_tail = self.tail.execute();
        tokio::join!(fut_head, fut_tail);
    }

    async fn spawn_strategy_tasks(&self, channels: &Arc<Vec<BoardCastChannel>>) {
        let HCons { head, tail } = self;
        let ch = channels.clone();
        let h = head.clone();

        tokio::spawn(async move {
            info!("Spawned strategy task for {}", h.strategy_name());
            strategy_handler_loop(h, &ch).await;
        });

        tail.spawn_strategy_tasks(channels).await;
    }
}

impl<Head, Tail> EventHandler for HCons<Head, Tail> where Head: Strategy, Tail: Strategy {}


impl<Head, Tail> AltEventHandler for HCons<Head, Tail>
where
    Head: AltEventHandler,
    Tail: AltEventHandler,
{
    async fn on_alt_event(&mut self, task_info: Arc<AltTaskInfo>) {
        let fut_head = self.head.on_alt_event(task_info.clone());
        let fut_tail = self.tail.on_alt_event(task_info);
        tokio::join!(fut_head, fut_tail);
    }

    async fn on_timer(&mut self){
        let fut_head = self.head.on_timer();
        let fut_tail = self.tail.on_timer();
        tokio::join!(fut_head, fut_tail);
    }
}

impl<Head, Tail> CexEventHandler for HCons<Head, Tail>
where
    Head: CexEventHandler,
    Tail: CexEventHandler,
{
    async fn on_cex_event(&mut self, task_info: Arc<WsTaskInfo>) {
        let fut_head = self.head.on_cex_event(task_info.clone());
        let fut_tail = self.tail.on_cex_event(task_info);
        tokio::join!(fut_head, fut_tail);
    }

    async fn on_candle(&mut self, msg: Arc<Vec<WsCandle>>) {
        let fut_head = self.head.on_candle(msg.clone());
        let fut_tail = self.tail.on_candle(msg);
        tokio::join!(fut_head, fut_tail);
    }
    async fn on_trade(&mut self, msg: Arc<Vec<WsTrade>>) {
        let fut_head = self.head.on_trade(msg.clone());
        let fut_tail = self.tail.on_trade(msg);
        tokio::join!(fut_head, fut_tail);
    }

    async fn on_lob(&mut self, msg: Arc<Vec<WsLob>>) {
        let fut_head = self.head.on_lob(msg.clone());
        let fut_tail = self.tail.on_lob(msg);
        tokio::join!(fut_head, fut_tail);
    }
}

impl<Head, Tail> DexEventHandler for HCons<Head, Tail> where Head: Strategy, Tail: Strategy {}


impl<Head, Tail> CommandEmitter for HCons<Head, Tail>
where
    Head: CommandEmitter,
    Tail: CommandEmitter,
{
    fn command_init(&mut self, command_handle: Arc<CommandHandle>) {
        self.head.command_init(command_handle.clone());
        self.tail.command_init(command_handle);
    }
}


