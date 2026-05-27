use std::sync::Arc;

use super::{task_alt::AltTaskInfo, task_ws::WsTaskInfo};

/// Runtime task declaration accepted by [`EnvBuilder`].
///
/// [`EnvBuilder`]: crate::arch::infra_core::env_builder::EnvBuilder
#[derive(Clone, Debug)]
pub enum TaskInfo {
    /// Non-websocket task.
    AltTask(Arc<AltTaskInfo>),
    /// Websocket relay task.
    WsTask(Arc<WsTaskInfo>),
}

#[derive(Clone, Debug)]
pub(crate) enum LogLevel {
    Info,
    Warn,
    Error,
}
