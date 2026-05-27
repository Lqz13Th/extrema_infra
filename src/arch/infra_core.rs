//! Runtime construction and execution.
//!
//! This module contains the environment builder and mediator. Use
//! [`env_builder::EnvBuilder`] in binaries to register broadcast channels,
//! tasks, and strategy modules, then call
//! [`env_mediator::EnvMediator::execute`] to start the runtime.
//!
//! The runtime owns task spawning and command-registry creation. Strategy
//! modules own business logic and receive typed events after the mediator has
//! initialized the environment.

pub mod env_builder;
pub(crate) mod env_core;
pub mod env_mediator;
