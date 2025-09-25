use std::sync::Arc;

use super::{
    task_alt::AltTaskInfo,
    task_ws::WsTaskInfo,
};

#[derive(Clone, Debug)]
pub enum TaskInfo {
    AltTask(Arc<AltTaskInfo>),
    WsTask(Arc<WsTaskInfo>),
}

#[derive(Clone, Debug)]
pub(crate) enum LogLevel {
    Info,
    Warn,
    Error,
}


