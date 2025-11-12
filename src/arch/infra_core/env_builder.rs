use std::sync::Arc;
use tracing::info;

use crate::arch::{
    infra_core::{
        env_core::EnvCore,
        env_mediator::EnvMediator,
    },
    strategy_base::{
        handler::handler_core::BoardCastChannel,
        hlist_core::{HCons, HNil},
    },
    task_execution::task_general::TaskInfo,
    traits::strategy::Strategy,
};

pub struct EnvBuilder<Strategies = HNil> {
    strategies: Strategies,
    board_cast_channels: Vec<BoardCastChannel>,
    tasks: Vec<TaskInfo>,
}

impl EnvBuilder<HNil> {
    pub fn new() -> Self {
        Self {
            strategies: HNil,
            board_cast_channels: Vec::new(),
            tasks: vec![],
        }
    }
}

impl Default for EnvBuilder<HNil> {
    fn default() -> Self {
        Self::new()
    }
}

impl<HeadList> EnvBuilder<HeadList> {
    pub fn with_board_cast_channel(mut self, channel: BoardCastChannel) -> Self {
        let channel_type_exists = self.board_cast_channels.iter().any(|ch| {
            std::mem::discriminant(ch) == std::mem::discriminant(&channel)
        });

        if !channel_type_exists {
            info!("Adding board cast channel: {:?}", channel);
            self.board_cast_channels.push(channel);
        } else {
            info!("Skipped duplicate channel: {:?}", channel);
        }

        self
    }
    
    pub fn with_strategy_module<S>(self, strategy: S) -> EnvBuilder<HCons<S, HeadList>>
    where
        S: Strategy + Clone,
    {
        info!("Adding strategy: {}", strategy.strategy_name());
        EnvBuilder {
            strategies: HCons {
                head: strategy,
                tail: self.strategies,
            },
            board_cast_channels: self.board_cast_channels,
            tasks: self.tasks,
        }
    }

    pub fn with_task(mut self, task: TaskInfo) -> Self {
        info!("Adding task: {:?}", task);
        self.tasks.push(task);
        self
    }
}

impl<Strategies> EnvBuilder<Strategies>
where
    Strategies: Strategy,
{
    pub fn build(self) -> EnvMediator<Strategies> {
        EnvMediator {
            core: EnvCore {
                strategy: self.strategies,
                channel: Arc::new(self.board_cast_channels),
            },
            tasks: self.tasks,
        }
    }
}
