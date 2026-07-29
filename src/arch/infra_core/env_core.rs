use std::sync::Arc;

use crate::arch::strategy_base::handler::task_channel::TaskChannels;

#[derive(Clone)]
pub(crate) struct EnvCore<S> {
    pub task_channels: Arc<TaskChannels>,
    pub strategy: S,
}
