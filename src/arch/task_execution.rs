//! Runtime task descriptors and task relay implementations.
//!
//! Tasks are the active workers owned by the runtime. Strategy modules do not
//! usually run their own polling loops; instead they declare tasks through
//! [`task_general::TaskInfo`] and receive events when those tasks publish into
//! broadcast channels.
//!
//! [`task_alt::AltTaskInfo`] describes non-websocket workers such as timers,
//! model prediction workers, instrument-intent relays, and order-execution
//! relays. [`task_ws::WsTaskInfo`] describes websocket relay workers. The
//! `register_*` modules contain the internal runtime machinery that turns those
//! descriptors into running tasks.

pub mod register_alt;
pub mod register_ws;
pub mod task_alt;
pub mod task_general;
pub mod task_ws;
