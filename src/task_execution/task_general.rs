use std::sync::Arc;

use crate::task_execution::alt_register::*;
use crate::task_execution::ws_register::*;

#[derive(Clone, Debug)]
pub enum TaskInfo {
    WsTask(Arc<WsTaskInfo>),
    AltTask(Arc<AltTaskInfo>),
}



