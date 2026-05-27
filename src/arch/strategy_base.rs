//! Strategy-side messaging primitives.
//!
//! This module contains the pieces that connect runtime tasks to strategy
//! callbacks:
//!
//! - [`handler`] defines typed broadcast messages and event payloads.
//! - [`command`] defines command handles, acknowledgements, and the registry
//!   used by strategies to send active commands back to tasks.
//! - [`hlist_core`] stores heterogeneous strategy modules without forcing them
//!   behind `Box<dyn Strategy>`.

pub mod command;
pub mod handler;
pub mod hlist_core;
