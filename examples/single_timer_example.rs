use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::{error, info, warn};

use extrema_infra::prelude::*;
use extrema_infra::market_assets::cex::prelude::*;

#[derive(Clone)]
struct EmptyStrategy;
impl Strategy for EmptyStrategy {
    async fn execute(&mut self) {
        info!("[EmptyStrategy] Executing...");
    }
    fn strategy_name(&self) -> &'static str { "EmptyStrategy" }
}
impl EventHandler for EmptyStrategy {}
impl AltEventHandler for EmptyStrategy {
    async fn on_timer(&mut self) {
        println!("[EmptyStrategy] Timer")
    }
}
impl CexEventHandler for EmptyStrategy {}
impl DexEventHandler for EmptyStrategy {}
impl CommandEmitter for EmptyStrategy {
    fn command_init(&mut self, _command_handle: Arc<CommandHandle>) {
        info!("[EmptyStrategy] Command channel initialized");
    }
}


#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Logger initialized");

    let alt_task = AltTaskInfo {
        alt_task_type: AltTaskType::TimerBasedState(5),
    };

    let mediator = EnvBuilder::new()
        .with_board_cast_channel(BoardCastChannel::default_timer())
        .with_strategy(EmptyStrategy)
        .with_task(TaskInfo::AltTask(Arc::new(alt_task)))
        .build();

    mediator.execute().await;
}
