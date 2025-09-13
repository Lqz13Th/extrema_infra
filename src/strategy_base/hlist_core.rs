use std::sync::Arc;
use futures_util::stream::SplitSink;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::info;
use tungstenite::Message;
use crate::strategy_base::event_notify::board_cast_channels::{strategy_board_cast_loop, BoardCastChannel};
use crate::task_execution::ws_register::{SharedWsWrite, WsTaskHandle};
use crate::strategy_base::event_notify::cex_notify::*;
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


impl Strategy for HNil {}
impl AltNotify for HNil {}
impl CexNotify for HNil {}
impl TaskOperation for HNil {}

impl<Head, Tail> Strategy for HCons<Head, Tail> where Head: Strategy, Tail: Strategy, {
    async fn spawn_strategy_tasks(&self, _channels: Arc<Vec<BoardCastChannel>>) {
        let HCons { head, tail } = self;
        let ch = _channels.clone();
        let h = head.clone();
        info!("Try spawn task for {}", h.name());

        tokio::spawn(async move {
            info!("Spawn task for {}", h.name());
            strategy_board_cast_loop(h, ch).await;
        });

        tail.spawn_strategy_tasks(_channels).await;
    }
}


impl<Head, Tail> AltNotify for HCons<Head, Tail> where Head: AltNotify, Tail: AltNotify, {
    fn on_timer(&mut self) -> impl Future<Output=()> + Send {
        let fut_head = self.head.on_timer();
        let fut_tail = self.tail.on_timer();
        async move { fut_head.await; fut_tail.await; }
    }
}

impl<Head, Tail> CexNotify for HCons<Head, Tail> where Head: CexNotify, Tail: CexNotify, {
    fn on_trade(&mut self, _msg: Arc<Vec<WsTrade>>) -> impl Future<Output=()> + Send {
        let fut_head = self.head.on_trade(_msg.clone());
        let fut_tail = self.tail.on_trade(_msg);
        async move { fut_head.await; fut_tail.await; }
    }

    fn on_lob(&mut self, _msg: Arc<Vec<WsLob>>) -> impl Future<Output=()> + Send {
        let fut_head = self.head.on_lob(_msg.clone());
        let fut_tail = self.tail.on_lob(_msg);
        async move { fut_head.await; fut_tail.await; }
    }
}

impl<Head, Tail> TaskOperation for HCons<Head, Tail>
where
    Head: TaskOperation,
    Tail: TaskOperation,
{
    fn on_ws_init(
        &mut self,
        _ws_task_handle: WsTaskHandle,
    ) -> impl Future<Output=()> + Send {
        let fut_head = self.head.on_ws_init(_ws_task_handle.clone());
        let fut_tail = self.tail.on_ws_init(_ws_task_handle);
        async move { fut_head.await; fut_tail.await; }
    }
}


