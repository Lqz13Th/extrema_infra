use std::{future::ready, sync::Arc};

use crate::arch::{
    strategy_base::{
        command::command_core::{CommandHandle, CommandRegistry},
        handler::{
            events::{InfraMsg, alt_events::*, lob_events::*},
            task_channel::TaskChannels,
        },
    },
    task_execution::{
        task_alt::{AltTaskInfo, AltTaskType},
        task_ws::{WsChannel, WsTaskInfo},
    },
};

/// Application logic registered in an [`EnvBuilder`].
///
/// A strategy module owns state and receives events through [`EventHandler`].
/// It also implements [`CommandEmitter`] so the runtime can provide command
/// handles for runtime tasks. Binaries may register one strategy module or many
/// independent modules in the same environment.
///
/// [`EnvBuilder`]: crate::arch::infra_core::env_builder::EnvBuilder
pub trait Strategy: CommandEmitter + EventHandler {
    /// Runs once before task workers are prepared and before event handlers start.
    ///
    /// Use this hook for config loading, API-key initialization, cache warmup,
    /// or local state restoration. It should complete; the runtime starts the
    /// strategy event loops after initialization.
    fn initialize(&mut self) -> impl Future<Output = ()> + Send;

    /// Human-readable module name used in runtime logs.
    fn strategy_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Internal hook used by the strategy-list runtime.
    ///
    /// Strategy modules registered through `EnvBuilder` normally should not
    /// override this method. The heterogeneous strategy list provides the
    /// runtime implementation that spawns each module's event handler loop.
    fn _spawn_strategy_tasks(
        &self,
        _task_channels: &Arc<TaskChannels>,
    ) -> impl Future<Output = ()> + Send {
        ready(())
    }
}

/// Outbound command surface for a strategy module.
///
/// `CommandEmitter` is the other half of the event-driven runtime. An
/// [`EventHandler`] receives inbound events from tasks; a `CommandEmitter`
/// sends outbound commands back to those tasks.
///
/// The lifecycle is:
///
/// 1. The strategy is constructed with an empty/default registry.
/// 2. [`EnvMediator::execute`] prepares all declared [`TaskInfo`] workers and
///    their command handles without starting producers.
/// 3. The runtime builds a [`CommandRegistry`] from those handles.
/// 4. The runtime calls [`CommandEmitter::command_init`] once on each strategy.
/// 5. Strategy receivers are created, then task workers are spawned.
/// 6. Event callbacks can call [`CommandEmitter::find_alt_handle`] or
///    [`CommandEmitter::find_ws_handle`] and then send [`TaskCommand`] values.
///    Those commands are how strategies actively request websocket
///    connect/login/subscribe/shutdown, order execution, intent publication, or
///    model prediction work.
///
/// Store the registry supplied by `command_init`. Returning a fresh
/// `CommandRegistry::default()` from [`CommandEmitter::command_registry`] loses
/// all task handles, so later `find_*_handle` calls will return `None`.
///
/// Command keys contain the complete [`AltTaskType`] or [`WsChannel`] plus a task
/// id. Embedded parameters such as scheduler duration or trade-stream type are
/// part of the key; websocket market is not. Event callback envelopes expose
/// only `task_id`, so `EnvBuilder` rejects id reuse within the same enum variant.
/// Different callback families, such as Trade and LOB, may reuse an id.
///
/// ```rust
/// use std::sync::Arc;
///
/// use extrema_infra::prelude::*;
///
/// #[derive(Clone)]
/// struct MyModule {
///     registry: Arc<CommandRegistry>,
/// }
///
/// impl CommandEmitter for MyModule {
///     fn command_init(&mut self, registry: Arc<CommandRegistry>) {
///         self.registry = registry;
///     }
///
///     fn command_registry(&self) -> Arc<CommandRegistry> {
///         self.registry.clone()
///     }
/// }
/// ```
///
/// [`EnvMediator::execute`]: crate::arch::infra_core::env_mediator::EnvMediator::execute
/// [`TaskInfo`]: crate::arch::task_execution::TaskInfo
/// See [`TaskCommand`] for the concrete command variants and what each command
/// does.
///
/// [`TaskCommand`]: crate::arch::strategy_base::command::command_core::TaskCommand
pub trait CommandEmitter: Clone + Send + Sync + 'static {
    /// Stores the command registry supplied by the runtime.
    ///
    /// This is called after task registration and before strategy event loops
    /// start. Implementations should usually just assign the `Arc` into the
    /// strategy state. Keep this method fast; use [`Strategy::initialize`] for
    /// startup IO and cache warmup.
    fn command_init(&mut self, registry: Arc<CommandRegistry>);

    /// Returns the strategy's current command registry.
    ///
    /// Return a clone of the stored `Arc<CommandRegistry>`. Do not construct a
    /// new registry here, because the new registry will not contain the runtime
    /// task handles.
    fn command_registry(&self) -> Arc<CommandRegistry>;

    /// Finds an alt-task command handle by task type and task id.
    ///
    /// Use this for scheduler/model/order/intent style tasks. For example, a
    /// strategy can find a `ModelPreds(..)` handle and send
    /// `TaskCommand::FeatInput`, or find an `OrderExecution` handle and send
    /// `TaskCommand::OrderExecute`.
    ///
    /// Returns `None` when no task with the same complete task type and task id
    /// was registered in `EnvBuilder`.
    fn find_alt_handle(
        &self,
        alt_task_type: &AltTaskType,
        task_id: u64,
    ) -> Option<Arc<CommandHandle>> {
        self.command_registry()
            .find_alt_handle(alt_task_type, task_id)
    }

    /// Finds a websocket-task command handle by websocket channel and task id.
    ///
    /// Websocket relays do not connect by themselves. A strategy usually reacts
    /// to [`EventHandler::on_ws_event`], finds the handle with this method,
    /// sends `TaskCommand::WsConnect`, and then sends login/subscription
    /// messages as `TaskCommand::WsMessage`.
    ///
    /// Returns `None` when no task with the same complete websocket channel and
    /// task id was registered in `EnvBuilder`.
    fn find_ws_handle(&self, channel: &WsChannel, task_id: u64) -> Option<Arc<CommandHandle>> {
        self.command_registry().find_ws_handle(channel, task_id)
    }
}

/// Inbound event callbacks for a strategy module.
///
/// `EventHandler` is deliberately callback-oriented because exchange-facing
/// systems are driven by independent event sources:
///
/// - websocket frames arrive when exchanges push market/account updates;
/// - scheduler ticks arrive at configured intervals;
/// - model prediction tasks emit when inference finishes;
/// - order and intent tasks emit when another module sends a command.
///
/// A strategy module implements only the callbacks it consumes. Every callback
/// defaults to no-op, so a module can be as narrow as "only account positions"
/// or as broad as "schedule + prices + account + orders". Task bindings
/// registered in `EnvBuilder` determine which concrete task streams reach each
/// strategy module.
///
/// Each callback receives an [`InfraMsg<T>`]. The payload is shared through
/// `Arc<T>`, and `msg.task_id` identifies the task instance that emitted the
/// event. Use `task_id` to route multiple markets, accounts, model workers, or
/// scheduler tasks that publish the same event type.
///
/// Keep callbacks responsive. The strategy event loop awaits a callback before
/// processing the next event for that strategy instance. Long-running work
/// should be moved into runtime tasks, spawned background work, or expressed as
/// commands through [`CommandEmitter`].
///
/// ```rust
/// use extrema_infra::prelude::*;
///
/// struct PositionLogger;
///
/// impl EventHandler for PositionLogger {
///     async fn on_acc_pos(&mut self, msg: InfraMsg<Vec<WsAccPosition>>) {
///         println!(
///             "task_id={} positions={}",
///             msg.task_id,
///             msg.data.len()
///         );
///     }
/// }
/// ```
pub trait EventHandler {
    /// Receives generic alt-task lifecycle/control events.
    ///
    /// Alt tasks emit this event before entering their main task distribution
    /// loop. Use it when a strategy needs to detect that an alt task exists and
    /// optionally send an initial command through [`CommandEmitter`].
    fn on_alt_event(&mut self, _msg: InfraMsg<AltTaskInfo>) -> impl Future<Output = ()> + Send {
        ready(())
    }

    /// Receives order execution batches.
    ///
    /// This is emitted by an `AltTaskType::OrderExecution` task after another
    /// strategy sends `TaskCommand::OrderExecute` to that task. Execution
    /// modules typically implement this callback.
    fn on_order_execution(
        &mut self,
        _msg: InfraMsg<Vec<AltOrder>>,
    ) -> impl Future<Output = ()> + Send {
        ready(())
    }

    /// Receives instrument, portfolio, or allocation intents.
    ///
    /// This is emitted by an `AltTaskType::InstIntent` task after a strategy
    /// sends `TaskCommand::InstIntent`. Allocators and portfolio mediators use
    /// this path to exchange target books or instrument-level intents.
    fn on_inst_intent(&mut self, _msg: InfraMsg<AltIntent>) -> impl Future<Output = ()> + Send {
        ready(())
    }

    /// Receives model prediction tensors from a model task.
    ///
    /// Model tasks emit this after receiving `TaskCommand::FeatInput` and
    /// finishing inference. The concrete model task publishes the result into
    /// its task-local broadcast ring.
    fn on_preds(&mut self, _msg: InfraMsg<AltTensor>) -> impl Future<Output = ()> + Send {
        ready(())
    }

    /// Receives periodic scheduler ticks.
    ///
    /// Scheduler tasks emit [`AltScheduleEvent`]. After task startup, their first
    /// tick is immediate and later ticks use the configured `Duration`. Use
    /// `msg.task_id` to distinguish multiple timers.
    fn on_schedule(&mut self, _msg: InfraMsg<AltScheduleEvent>) -> impl Future<Output = ()> + Send {
        ready(())
    }

    /// Receives generic websocket-task lifecycle/control events.
    ///
    /// Websocket tasks emit this before every connection or reconnection cycle
    /// while waiting for `TaskCommand::WsConnect`. Implementations should be
    /// safe to call repeatedly and must reconnect, authenticate, and subscribe
    /// the relay for each cycle.
    fn on_ws_event(&mut self, _msg: InfraMsg<WsTaskInfo>) -> impl Future<Output = ()> + Send {
        ready(())
    }

    /// Receives normalized public trade batches.
    ///
    /// A concrete websocket task configured with [`WsChannel::Trades`] publishes
    /// each batch into its task-local broadcast ring.
    fn on_trade(&mut self, _msg: InfraMsg<Vec<WsTrade>>) -> impl Future<Output = ()> + Send {
        ready(())
    }

    /// Receives normalized order book updates.
    ///
    /// A concrete exchange task that implements [`WsChannel::Lob`] routing
    /// publishes each update into its task-local broadcast ring.
    fn on_lob(&mut self, _msg: InfraMsg<Vec<WsLob>>) -> impl Future<Output = ()> + Send {
        ready(())
    }

    /// Receives normalized market-by-order order book updates.
    ///
    /// A concrete exchange task that implements [`WsChannel::LobMbo`] routing
    /// publishes each update into its task-local broadcast ring.
    fn on_lob_mbo(&mut self, _msg: InfraMsg<Vec<WsLobMbo>>) -> impl Future<Output = ()> + Send {
        ready(())
    }

    /// Receives normalized candle batches.
    ///
    /// A concrete websocket task configured with [`WsChannel::Candles`]
    /// publishes each batch into its task-local broadcast ring.
    fn on_candle(&mut self, _msg: InfraMsg<Vec<WsCandle>>) -> impl Future<Output = ()> + Send {
        ready(())
    }

    /// Receives private account order updates.
    ///
    /// Emitted by private websocket relays configured with
    /// [`WsChannel::AccountOrders`]. Use this for order status, fills,
    /// cancellations, and rejection events normalized by the exchange client.
    fn on_acc_order(&mut self, _msg: InfraMsg<Vec<WsAccOrder>>) -> impl Future<Output = ()> + Send {
        ready(())
    }

    /// Receives private account balance and position updates.
    ///
    /// Emitted by private websocket relays configured with
    /// [`WsChannel::AccountBalAndPos`]. Use this when the exchange provides
    /// balance and position state in the same stream.
    fn on_acc_bal_pos(
        &mut self,
        _msg: InfraMsg<Vec<WsAccBalPos>>,
    ) -> impl Future<Output = ()> + Send {
        ready(())
    }

    /// Receives private account position-only updates.
    ///
    /// Emitted by private websocket relays configured with
    /// [`WsChannel::AccountPositions`]. Portfolio and risk modules typically
    /// use this callback to keep local account state fresh.
    fn on_acc_pos(
        &mut self,
        _msg: InfraMsg<Vec<WsAccPosition>>,
    ) -> impl Future<Output = ()> + Send {
        ready(())
    }
}
