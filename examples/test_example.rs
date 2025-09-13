use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::info;
use extrema_infra::traits::strategy::*;
use extrema_infra::infra_core::{
    env_builder::*,
    env_mediator::*,
};
use extrema_infra::strategy_base::event_notify::board_cast_channels::*;
use extrema_infra::task_execution::alt_register::*;
use extrema_infra::task_execution::alt_register::AltTaskType::TimerBasedState;
use extrema_infra::task_execution::general_register::*;

// 策略 1
#[derive(Clone)]
struct StrategyA;

impl AltNotify for StrategyA {
    async fn on_timer(&mut self) {
        println!("StrategyA timer triggered");
    }
}

impl CexNotify for StrategyA {}
impl TaskOperation for StrategyA {}

impl Strategy for StrategyA {
    fn name(&self) -> &'static str { "StrategyA" }
}

// 策略 2
#[derive(Clone)]
struct StrategyB;

impl AltNotify for StrategyB {
    async fn on_timer(&mut self) {
        println!("StrategyB timer triggered");
    }
}

impl CexNotify for StrategyB {}
impl TaskOperation for StrategyB {}

impl Strategy for StrategyB {
    fn name(&self) -> &'static str { "StrategyB" }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Logger initialized");

    let alt_task = AltTaskInfo {
        alt_task_type: TimerBasedState(1000),
    };

    let mediator = EnvBuilder::new()
        .with_board_cast_channel(BoardCastChannel::default_timer())
        .with_strategy(StrategyA)
        .with_strategy(StrategyB)
        .with_task(TaskInfo::AltTask(alt_task))
        .build();

    mediator.execute().await;
}
