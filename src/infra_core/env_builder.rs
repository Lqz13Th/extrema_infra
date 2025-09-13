use std::sync::Arc;
use tracing::info;

use crate::infra_core::{
    env_core::EnvCore,
    env_mediator::EnvMediator,
};
use crate::strategy_base::{
    event_notify::board_cast_channels::BoardCastChannel,
    hlist_core::{
        HCons, 
        HNil
    },
};
use crate::task_execution::general_register::TaskInfo;
use crate::traits::strategy::Strategy;

pub struct EnvBuilder<Strategies = HNil> {
    strategies: Strategies,
    board_cast_channels: Vec<BoardCastChannel>,
    tasks: Vec<TaskInfo>,
}

impl EnvBuilder<HNil> {
    pub fn new() -> Self {
        Self {
            strategies: HNil,
            board_cast_channels: vec![],
            tasks: vec![],
        }
    }
}

impl<HeadList> EnvBuilder<HeadList> {
    pub fn with_board_cast_channel(mut self, channel: BoardCastChannel) -> Self {
        self.board_cast_channels.push(channel);
        self
    }
    

    pub fn with_strategy<S>(self, strategy: S) -> EnvBuilder<HCons<S, HeadList>>
    where
        S: Strategy + Clone,
    {
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
    Strategies: Strategy + Clone,
{
    pub fn build(self) -> EnvMediator<Strategies> {
        EnvMediator {
            core: EnvCore {
                strategies: self.strategies,
                board_cast_channels: Arc::new(self.board_cast_channels),
            },
            tasks: self.tasks,
        }
    }
}
