//! Runtime task descriptors and task relay implementations.
//!
//! Tasks are the active workers owned by the runtime. Strategy modules do not
//! usually run their own polling loops; instead they declare tasks through
//! [`TaskInfo`] and receive events when those tasks publish into
//! their task-local broadcast streams.
//!
//! [`task_alt::AltTaskInfo`] describes non-websocket workers such as timers,
//! model prediction workers, instrument-intent relays, and order-execution
//! relays. [`task_ws::WsTaskInfo`] describes websocket relay workers. The
//! `*_runner` modules contain the internal runtime machinery that turns those
//! descriptors into running tasks.

pub(crate) mod alt_runner;
pub mod task_alt;
pub mod task_general;
pub mod task_ws;
pub(crate) mod ws_runner;

pub use task_general::{TaskInfo, TaskKey};
