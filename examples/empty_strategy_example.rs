use std::{
    sync::Arc,
    time::Duration,
};
use tracing::info;
use extrema_infra::prelude::*;

#[derive(Clone)]
struct EmptyStrategy;
impl EventHandler for EmptyStrategy {}
impl CexEventHandler for EmptyStrategy {}
impl DexEventHandler for EmptyStrategy {}

impl Strategy for EmptyStrategy {
    async fn initialize(&mut self) {
        info!("[EmptyStrategy] Executing...");
    }
    fn strategy_name(&self) -> &'static str { "EmptyStrategy" }
}

impl AltEventHandler for EmptyStrategy {
    async fn on_schedule(
        &mut self,
        msg: InfraMsg<AltScheduleEvent>,
    ) {
        info!("[EmptyStrategy] AltEventHandler: {:?}", msg);
    }
}

impl CommandEmitter for EmptyStrategy {
    fn command_init(&mut self, _command_handle: Arc<CommandHandle>) {
        info!("[EmptyStrategy] Command channel initialized");
    }

    fn command_registry(&self) -> Vec<Arc<CommandHandle>> {
        Vec::new()
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Logger initialized");

    let alt_task = AltTaskInfo {
        alt_task_type: AltTaskType::TimeScheduler(Duration::from_secs(5)),
        chunk: 1,
        task_id: None,
    };

    let env = EnvBuilder::new()
        .with_board_cast_channel(BoardCastChannel::default_alt_event())
        .with_board_cast_channel(BoardCastChannel::default_scheduler())
        .with_strategy_module(EmptyStrategy)
        .with_task(TaskInfo::AltTask(Arc::new(alt_task)))
        .build();

    env.execute().await;
}
