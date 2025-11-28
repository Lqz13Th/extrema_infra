use std::sync::Arc;
use tracing::info;

use crate::arch::{
    market_assets::api_general::OrderParams,
    strategy_base::{
        command::command_core::CommandHandle,
        handler::{
            handler_core::*,
            alt_events::*,
            cex_events::*,
        }
    },
    task_execution::{
        task_alt::AltTaskInfo,
        task_ws::WsTaskInfo,
    },
    traits::strategy::*,
};

#[derive(Clone)]
pub struct HNil;

impl Strategy for HNil { async fn initialize(&mut self) {} }
impl CommandEmitter for HNil {
    fn command_init(&mut self, _command_handle: Arc<CommandHandle>) {}
    fn command_registry(&self) -> Vec<Arc<CommandHandle>> { Vec::new() }
}
impl EventHandler for HNil {}

#[derive(Clone)]
pub struct HCons<Head, Tail> {
    pub head: Head,
    pub tail: Tail,
}

impl<Head, Tail> Strategy for HCons<Head, Tail>
where
    Head: Strategy + Send + Sync + Clone + 'static,
    Tail: Strategy + Send + Sync + Clone + 'static,
{
    async fn initialize(&mut self) {
        let fut_head = self.head.initialize();
        let fut_tail = self.tail.initialize();
        tokio::join!(fut_head, fut_tail);
    }

    async fn _spawn_strategy_tasks(&self, channels: &Arc<Vec<BoardCastChannel>>) {
        let HCons { head, tail } = self;
        let ch = channels.clone();
        let h = head.clone();

        tokio::spawn(async move {
            info!("Spawned strategy task for {}", h.strategy_name());
            strategy_handler_loop(h, &ch).await;
        });

        tail._spawn_strategy_tasks(channels).await;
    }
}
impl<Head, Tail> CommandEmitter for HCons<Head, Tail>
where
    Head: CommandEmitter,
    Tail: CommandEmitter,
{
    fn command_init(&mut self, command_handle: Arc<CommandHandle>) {
        self.head.command_init(command_handle.clone());
        self.tail.command_init(command_handle);
    }

    fn command_registry(&self) -> Vec<Arc<CommandHandle>> {
        let mut all = Vec::new();
        all.extend(self.head.command_registry());
        all.extend(self.tail.command_registry());
        all
    }
}

impl<Head, Tail> EventHandler for HCons<Head, Tail>
where
    Head: Strategy,
    Tail: Strategy,
{
    async fn on_alt_event(&mut self, task_info: InfraMsg<AltTaskInfo>) {
        let fut_head = self.head.on_alt_event(task_info.clone());
        let fut_tail = self.tail.on_alt_event(task_info);
        tokio::join!(fut_head, fut_tail);
    }

    async fn on_order_execution(&mut self, msg: InfraMsg<Vec<OrderParams>>) {
        let fut_head = self.head.on_order_execution(msg.clone());
        let fut_tail = self.tail.on_order_execution(msg);
        tokio::join!(fut_head, fut_tail);
    }

    async fn on_schedule(&mut self, msg: InfraMsg<AltScheduleEvent>) {
        let fut_head = self.head.on_schedule(msg.clone());
        let fut_tail = self.tail.on_schedule(msg);
        tokio::join!(fut_head, fut_tail);
    }

    async fn on_preds(&mut self, msg: InfraMsg<AltTensor>) {
        let fut_head = self.head.on_preds(msg.clone());
        let fut_tail = self.tail.on_preds(msg);
        tokio::join!(fut_head, fut_tail);
    }

    async fn on_ws_event(&mut self, task_info: InfraMsg<WsTaskInfo>) {
        let fut_head = self.head.on_ws_event(task_info.clone());
        let fut_tail = self.tail.on_ws_event(task_info);
        tokio::join!(fut_head, fut_tail);
    }

    async fn on_trade(&mut self, msg: InfraMsg<Vec<WsTrade>>) {
        let fut_head = self.head.on_trade(msg.clone());
        let fut_tail = self.tail.on_trade(msg);
        tokio::join!(fut_head, fut_tail);
    }

    async fn on_lob(&mut self, msg: InfraMsg<Vec<WsLob>>) {
        let fut_head = self.head.on_lob(msg.clone());
        let fut_tail = self.tail.on_lob(msg);
        tokio::join!(fut_head, fut_tail);
    }

    async fn on_candle(&mut self, msg: InfraMsg<Vec<WsCandle>>) {
        let fut_head = self.head.on_candle(msg.clone());
        let fut_tail = self.tail.on_candle(msg);
        tokio::join!(fut_head, fut_tail);
    }

    async fn on_acc_order(&mut self, msg: InfraMsg<Vec<WsAccOrder>>) {
        let fut_head = self.head.on_acc_order(msg.clone());
        let fut_tail = self.tail.on_acc_order(msg);
        tokio::join!(fut_head, fut_tail);
    }

    async fn on_acc_bal_pos(&mut self, msg: InfraMsg<Vec<WsAccBalPos>>) {
        let fut_head = self.head.on_acc_bal_pos(msg.clone());
        let fut_tail = self.tail.on_acc_bal_pos(msg);
        tokio::join!(fut_head, fut_tail);
    }
}




