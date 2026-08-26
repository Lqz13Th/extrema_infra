use std::sync::Arc;

pub mod alt_events;
pub mod lob_events;
pub mod ws_events;

pub use alt_events::*;
pub use lob_events::*;
pub use ws_events::*;

/// Message envelope published by one runtime task.
#[derive(Clone, Debug)]
pub struct InfraMsg<T> {
    /// Runtime task id that emitted this message.
    pub task_id: u64,
    /// Shared event payload.
    pub data: Arc<T>,
}
