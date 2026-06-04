use std::{future::ready, sync::Arc};

use crate::arch::{
    strategy_base::{
        command::command_core::{CommandHandle, CommandRegistry},
        handler::{
            alt_events::*,
            event_mask::EventMask,
            handler_core::{BoardCastChannel, InfraMsg},
            lob_events::*,
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
/// handles for spawned tasks. Binaries may register one strategy module or many
/// independent modules in the same environment.
///
/// [`EnvBuilder`]: crate::arch::infra_core::env_builder::EnvBuilder
pub trait Strategy: CommandEmitter + EventHandler {
    /// Runs once before tasks are registered and before event handlers start.
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
        _channels: &Arc<Vec<BoardCastChannel>>,
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
/// 2. [`EnvMediator::execute`] spawns all declared [`TaskInfo`] workers.
/// 3. The runtime builds a [`CommandRegistry`] from those workers.
/// 4. The runtime calls [`CommandEmitter::command_init`] once on each strategy.
/// 5. Event callbacks can call [`CommandEmitter::find_alt_handle`] or
///    [`CommandEmitter::find_ws_handle`] and then send [`TaskCommand`] values.
///    Those commands are how strategies actively request websocket
///    connect/login/subscribe/shutdown, order execution, intent publication, or
///    model prediction work.
///
/// Store the registry supplied by `command_init`. Returning a fresh
/// `CommandRegistry::default()` from [`CommandEmitter::command_registry`] loses
/// all task handles, so later `find_*_handle` calls will return `None`.
///
/// Command keys are not market-aware. Alt-task handles are keyed by
/// `(AltTaskType, task_id)`, and websocket handles are keyed by
/// `(WsChannel, task_id)`. If two tasks use the same task type/channel, give
/// them distinct task ids even when they target different markets or accounts.
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
/// [`TaskInfo`]: crate::arch::task_execution::task_general::TaskInfo
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
    /// Returns `None` when no task with the same `(AltTaskType, task_id)` was
    /// registered in `EnvBuilder`.
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
    /// Returns `None` when no task with the same `(WsChannel, task_id)` was
    /// registered in `EnvBuilder`.
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
/// or as broad as "schedule + prices + account + orders". By default,
/// [`EventHandler::event_mask`] subscribes to every registered channel for
/// backwards compatibility; latency-sensitive modules can override it to avoid
/// receiver creation and wakeups for unused callbacks.
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
/// Common callback/channel pairs:
///
/// | Callback | Typical source | Required channel | Event mask |
/// | --- | --- | --- | --- |
/// | [`EventHandler::on_schedule`] | `AltTaskType::TimeScheduler` | `BoardCastChannel::default_scheduler()` | `EventMask::SCHEDULE` |
/// | [`EventHandler::on_inst_intent`] | `TaskCommand::InstIntent` | `BoardCastChannel::default_inst_intent()` | `EventMask::INST_INTENT` |
/// | [`EventHandler::on_order_execution`] | `TaskCommand::OrderExecute` | `BoardCastChannel::default_order_execution()` | `EventMask::ORDER_EXECUTION` |
/// | [`EventHandler::on_preds`] | `TaskCommand::FeatInput` to a model task | `BoardCastChannel::default_model_preds()` | `EventMask::MODEL_PREDS` |
/// | [`EventHandler::on_ws_event`] | websocket relay startup/control | `BoardCastChannel::default_ws_event()` | `EventMask::WS_EVENT` |
/// | [`EventHandler::on_trade`] | public trade websocket relay | `BoardCastChannel::default_trade()` | `EventMask::TRADE` |
/// | [`EventHandler::on_lob`] | public LOB websocket relay | `BoardCastChannel::default_lob()` | `EventMask::LOB` |
/// | [`EventHandler::on_lob_mbo`] | public market-by-order websocket relay | `BoardCastChannel::default_lob_mbo()` | `EventMask::LOB_MBO` |
/// | [`EventHandler::on_candle`] | public candle websocket relay | `BoardCastChannel::default_candle()` | `EventMask::CANDLE` |
/// | [`EventHandler::on_acc_order`] | private account-order websocket relay | `BoardCastChannel::default_account_order()` | `EventMask::ACC_ORDER` |
/// | [`EventHandler::on_acc_bal_pos`] | private balance/position websocket relay | `BoardCastChannel::default_account_bal_pos()` | `EventMask::ACC_BAL_POS` |
/// | [`EventHandler::on_acc_pos`] | private position websocket relay | `BoardCastChannel::default_account_pos()` | `EventMask::ACC_POS` |
///
/// ```rust
/// use extrema_infra::prelude::*;
///
/// struct PositionLogger;
///
/// impl EventHandler for PositionLogger {
///     fn event_mask(&self) -> EventMask {
///         EventMask::ACC_POS
///     }
///
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
    /// Declares which runtime event streams this module wants to subscribe to.
    ///
    /// The default is [`EventMask::ALL`] so existing strategies keep receiving
    /// every registered broadcast channel. Override this in latency-sensitive
    /// modules to avoid receiver creation and wakeups for unused callbacks. This
    /// is read once when the strategy event loop starts.
    fn event_mask(&self) -> EventMask {
        EventMask::ALL
    }

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
    /// finishing inference. Add `BoardCastChannel::default_model_preds()` when
    /// registering model tasks.
    fn on_preds(&mut self, _msg: InfraMsg<AltTensor>) -> impl Future<Output = ()> + Send {
        ready(())
    }

    /// Receives periodic scheduler ticks.
    ///
    /// Scheduler tasks emit [`AltScheduleEvent`] at their configured
    /// `Duration`. Use `msg.task_id` to distinguish multiple timers.
    fn on_schedule(&mut self, _msg: InfraMsg<AltScheduleEvent>) -> impl Future<Output = ()> + Send {
        ready(())
    }

    /// Receives generic websocket-task lifecycle/control events.
    ///
    /// Websocket tasks emit this before waiting for the initial
    /// `TaskCommand::WsConnect`. Strategies commonly implement this callback to
    /// connect, authenticate, and subscribe the relay.
    fn on_ws_event(&mut self, _msg: InfraMsg<WsTaskInfo>) -> impl Future<Output = ()> + Send {
        ready(())
    }

    /// Receives normalized public trade batches.
    ///
    /// Emitted by websocket relays configured with [`WsChannel::Trades`] and a
    /// `BoardCastChannel::default_trade()` broadcast channel.
    fn on_trade(&mut self, _msg: InfraMsg<Vec<WsTrade>>) -> impl Future<Output = ()> + Send {
        ready(())
    }

    /// Receives normalized order book updates.
    ///
    /// Emitted by exchange relays that implement [`WsChannel::Lob`] routing and
    /// publish into `BoardCastChannel::default_lob()`.
    fn on_lob(&mut self, _msg: InfraMsg<Vec<WsLob>>) -> impl Future<Output = ()> + Send {
        ready(())
    }

    /// Receives normalized market-by-order order book updates.
    ///
    /// Emitted by exchange relays that implement [`WsChannel::LobMbo`] routing
    /// and publish into `BoardCastChannel::default_lob_mbo()`.
    fn on_lob_mbo(&mut self, _msg: InfraMsg<Vec<WsLobMbo>>) -> impl Future<Output = ()> + Send {
        ready(())
    }

    /// Receives normalized candle batches.
    ///
    /// Emitted by websocket relays configured with [`WsChannel::Candles`] and a
    /// `BoardCastChannel::default_candle()` broadcast channel.
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
