//! Event-driven infrastructure for exchange-facing trading systems.
//!
//! `extrema_infra` provides the runtime layer used by strategy binaries that
//! need to combine timers, model workers, order execution, public market data,
//! and private account streams in one process. The framework is intentionally
//! small at the strategy boundary: implement [`Strategy`], register the tasks
//! your process needs, add the broadcast channels that should publish task
//! output, then start the environment with [`EnvBuilder`].
//!
//! The crate is organized around a few concepts:
//!
//! - [`Strategy`] is the unit of application logic. A binary can register one
//!   strategy module or several independent modules in the same runtime.
//! - [`EventHandler`] contains async callbacks for schedule ticks, model
//!   predictions, order execution intents, trades, LOB updates, candles, and
//!   private account updates. All callbacks default to no-op.
//! - [`CommandEmitter`] gives a strategy access to task command handles after
//!   the runtime has spawned those tasks.
//! - [`TaskInfo`] declares work that the runtime owns, such as [`AltTaskInfo`]
//!   for scheduler/model/order-intent work or [`WsTaskInfo`] for websocket
//!   relays.
//! - [`BoardCastChannel`] declares which event streams are available to
//!   strategies. Add only the channels you want to consume or publish.
//! - [`prelude`] re-exports the common types used by strategy binaries.
//!
//! # Why Events Drive Strategies
//!
//! Trading systems are naturally event-driven. Market data arrives whenever an
//! exchange pushes a websocket frame. Private account streams update after
//! fills, cancellations, balance changes, and position changes. Timers trigger
//! periodic tasks such as refresh, rebalancing, risk checks, model inference, or
//! state snapshots. These inputs do not share one clean loop cadence, and they
//! should not force every strategy module to poll every data source.
//!
//! `extrema_infra` separates those responsibilities:
//!
//! - Runtime tasks own IO, timers, model workers, and order relays.
//! - Tasks publish normalized messages into typed broadcast channels.
//! - Strategy modules implement event callbacks for only the streams they care
//!   about.
//! - Strategy modules send commands back to tasks through command handles when
//!   they need active work such as websocket connect/subscribe or order
//!   execution.
//!
//! That shape keeps strategy logic reactive and local: a portfolio module can
//! handle account-position events, an allocator can handle target-weight
//! intents, an execution module can handle order batches, and all of them can
//! live in the same runtime without owning duplicate websocket or timer loops.
//!
//! # Core Trait Responsibilities
//!
//! [`Strategy`] is the lifecycle trait. It has one required startup hook,
//! `initialize`, which should complete after loading config, initializing
//! clients, or warming local state. Runtime event loops start after this phase.
//!
//! [`EventHandler`] is the inbound event surface. Its methods are callbacks:
//! `on_schedule`, `on_trade`, `on_candle`, `on_acc_pos`, `on_inst_intent`,
//! `on_order_execution`, and so on. Every callback defaults to no-op, so modules
//! stay narrow and only implement the events that matter to them.
//!
//! [`CommandEmitter`] is the outbound command surface. After tasks are spawned,
//! the runtime supplies a [`CommandRegistry`]. Strategy modules store that
//! registry, then find task handles by `(task type, task id)` or
//! `(websocket channel, task id)` when they need to send commands.
//!
//! [`MarketLobApi`], [`LobPublicRest`], [`LobPrivateRest`], and
//! [`LobWebsocket`] define exchange-client adapters. Implementations translate
//! exchange-specific REST and websocket details into the normalized data types
//! consumed by strategy modules.
//!
//! # Event Flow
//!
//! A typical runtime flow looks like this:
//!
//! ```text
//! EnvBuilder
//!   -> registers BoardCastChannel values
//!   -> registers TaskInfo values
//!   -> registers Strategy modules
//!   -> EnvMediator::execute()
//!       -> Strategy::initialize()
//!       -> spawn AltTask/WsTask workers
//!       -> CommandEmitter::command_init()
//!       -> spawn strategy event loops
//!       -> tasks publish InfraMsg<T>
//!       -> EventHandler callbacks react
//!       -> strategies send TaskCommand through CommandHandle when needed
//! ```
//!
//! `InfraMsg<T>` always carries the `task_id` that emitted the event. Use it to
//! route multiple accounts, markets, model workers, or schedule tasks that
//! publish into the same event type.
//!
//! # Minimal scheduler runtime
//!
//! ```rust,no_run
//! use std::{sync::Arc, time::Duration};
//!
//! use extrema_infra::prelude::*;
//!
//! #[derive(Clone)]
//! struct Heartbeat {
//!     registry: Arc<CommandRegistry>,
//! }
//!
//! impl Heartbeat {
//!     fn new() -> Self {
//!         Self {
//!             registry: Arc::new(CommandRegistry::default()),
//!         }
//!     }
//! }
//!
//! impl Strategy for Heartbeat {
//!     async fn initialize(&mut self) {
//!         // Load config, warm local state, or initialize API clients here.
//!     }
//! }
//!
//! impl CommandEmitter for Heartbeat {
//!     fn command_init(&mut self, registry: Arc<CommandRegistry>) {
//!         self.registry = registry;
//!     }
//!
//!     fn command_registry(&self) -> Arc<CommandRegistry> {
//!         self.registry.clone()
//!     }
//! }
//!
//! impl EventHandler for Heartbeat {
//!     async fn on_schedule(&mut self, msg: InfraMsg<AltScheduleEvent>) {
//!         println!("schedule tick from task_id={}", msg.task_id);
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let schedule = AltTaskInfo {
//!         alt_task_type: AltTaskType::TimeScheduler(Duration::from_secs(5)),
//!         chunk: 1,
//!         task_base_id: Some(1),
//!     };
//!
//!     let env = EnvBuilder::new()
//!         .with_board_cast_channel(BoardCastChannel::default_alt_event())
//!         .with_board_cast_channel(BoardCastChannel::default_scheduler())
//!         .with_task(TaskInfo::AltTask(Arc::new(schedule)))
//!         .with_strategy_module(Heartbeat::new())
//!         .build();
//!
//!     env.execute().await;
//! }
//! ```
//!
//! # TLS initialization
//!
//! Binaries that use TLS-backed REST or websocket clients must install a
//! `rustls` crypto provider before creating those clients:
//!
//! ```rust,no_run
//! rustls::crypto::aws_lc_rs::default_provider()
//!     .install_default()
//!     .expect("failed to install rustls crypto provider");
//! ```
//!
//! # Downstream usage shape
//!
//! Real strategy binaries usually follow the same shape as the minimal example:
//! build `AltTaskInfo` and `WsTaskInfo` values from config, add the matching
//! broadcast channels, register one or more strategy modules, and call
//! [`EnvMediator::execute`]. A simple signal process may only need scheduler and
//! intent channels; an execution orchestrator typically adds private account
//! websocket channels, order execution tasks, risk/evaluation modules, and
//! several independent strategy modules.
//!
//! [`Strategy`]: crate::arch::traits::strategy::Strategy
//! [`EventHandler`]: crate::arch::traits::strategy::EventHandler
//! [`CommandEmitter`]: crate::arch::traits::strategy::CommandEmitter
//! [`EnvBuilder`]: crate::arch::infra_core::env_builder::EnvBuilder
//! [`EnvMediator::execute`]: crate::arch::infra_core::env_mediator::EnvMediator::execute
//! [`TaskInfo`]: crate::arch::task_execution::task_general::TaskInfo
//! [`AltTaskInfo`]: crate::arch::task_execution::task_alt::AltTaskInfo
//! [`WsTaskInfo`]: crate::arch::task_execution::task_ws::WsTaskInfo
//! [`BoardCastChannel`]: crate::arch::strategy_base::handler::handler_core::BoardCastChannel
//! [`CommandRegistry`]: crate::arch::strategy_base::command::command_core::CommandRegistry
//! [`MarketLobApi`]: crate::arch::traits::market_lob::MarketLobApi
//! [`LobPublicRest`]: crate::arch::traits::market_lob::LobPublicRest
//! [`LobPrivateRest`]: crate::arch::traits::market_lob::LobPrivateRest
//! [`LobWebsocket`]: crate::arch::traits::market_lob::LobWebsocket
#![doc = include_str!("../docs/usage.md")]
pub mod errors;
pub mod prelude;
pub mod arch {
    pub mod infra_core;
    pub mod market_assets;
    pub mod strategy_base;
    pub mod task_execution;
    pub mod traits;
}
