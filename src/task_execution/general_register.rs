use crate::task_execution::alt_register::*;
use crate::task_execution::ws_register::*;

#[derive(Clone, Debug)]
pub enum TaskInfo {
    WsTask(WsTaskInfo),
    AltTask(AltTaskInfo),
}



