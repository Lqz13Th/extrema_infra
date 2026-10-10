//! Strategy-side messaging primitives.
//!
//! This module contains the pieces that connect runtime tasks to strategy
//! callbacks:
//!
//! - [`handler`] defines typed broadcast messages and event payloads.
//! - [`command`] defines command handles, acknowledgements, and the registry
//!   used by strategies to send active commands back to tasks.
//! - [`hlist_core`] stores heterogeneous strategy modules and the registered
//!   websocket decoders without forcing them behind `Box<dyn ...>`.
//! - [`strategy_module`] and [`strategy_group`] wrap one module or a group of
//!   same-type modules with their optional task bindings.

pub mod command;
pub mod handler;
pub mod hlist_core;
pub mod strategy_group;
pub mod strategy_module;
