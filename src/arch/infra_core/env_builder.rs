use std::sync::Arc;
use tracing::info;

use crate::arch::{
    infra_core::{env_core::EnvCore, env_mediator::EnvMediator},
    strategy_base::{
        handler::handler_core::BoardCastChannel,
        hlist_core::{HCons, HNil},
        strategy_group::InnerStrategyGroup,
        strategy_module::InnerStrategyModule,
    },
    task_execution::{task_alt::AltTaskType, task_general::TaskInfo, task_ws::WsChannel},
    traits::strategy::Strategy,
};

/// Builder for an `extrema_infra` runtime.
///
/// Use this builder in the final binary to declare runtime tasks and strategy
/// modules. `build` infers the default broadcast channels required by each
/// task. Call `with_board_cast_channel` only when a stream needs an explicit
/// channel or custom capacity. `with_strategy_module` keeps each strategy as
/// its concrete type by accumulating modules in a heterogeneous list, so a
/// process can compose different modules without boxing them behind a trait
/// object.
///
/// ```rust,no_run
/// use std::{sync::Arc, time::Duration};
///
/// use extrema_infra::prelude::*;
///
/// let task = AltTaskInfo {
///     alt_task_type: AltTaskType::TimeScheduler(Duration::from_secs(5)),
///     chunk: 1,
///     task_base_id: Some(1),
/// };
///
/// # #[derive(Clone)]
/// # struct MyStrategy;
/// # impl Strategy for MyStrategy { async fn initialize(&mut self) {} }
/// # impl CommandEmitter for MyStrategy {
/// #     fn command_init(&mut self, _: Arc<CommandRegistry>) {}
/// #     fn command_registry(&self) -> Arc<CommandRegistry> {
/// #         Arc::new(CommandRegistry::default())
/// #     }
/// # }
/// # impl EventHandler for MyStrategy {}
/// # let my_strategy = MyStrategy;
/// let env = EnvBuilder::new()
///     .with_task(TaskInfo::AltTask(Arc::new(task)))
///     .with_strategy_module(my_strategy)
///     .build();
/// ```
pub struct EnvBuilder<Strategies = HNil> {
    board_cast_channels: Vec<BoardCastChannel>,
    tasks: Vec<TaskInfo>,
    strategies: Strategies,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChannelKind {
    Alt,
    Ws,
    OrderExecute,
    InstIntent,
    ModelPreds,
    Schedule,
    Trade,
    Lob,
    LobMbo,
    Candle,
    AccOrder,
    AccBalPos,
    AccPos,
}

impl ChannelKind {
    fn of(channel: &BoardCastChannel) -> Self {
        match channel {
            BoardCastChannel::Alt(_) => Self::Alt,
            BoardCastChannel::Ws(_) => Self::Ws,
            BoardCastChannel::OrderExecute(_) => Self::OrderExecute,
            BoardCastChannel::InstIntent(_) => Self::InstIntent,
            BoardCastChannel::ModelPreds(_) => Self::ModelPreds,
            BoardCastChannel::Schedule(_) => Self::Schedule,
            BoardCastChannel::Trade(_) => Self::Trade,
            BoardCastChannel::Lob(_) => Self::Lob,
            BoardCastChannel::LobMbo(_) => Self::LobMbo,
            BoardCastChannel::Candle(_) => Self::Candle,
            BoardCastChannel::AccOrder(_) => Self::AccOrder,
            BoardCastChannel::AccBalPos(_) => Self::AccBalPos,
            BoardCastChannel::AccPos(_) => Self::AccPos,
        }
    }

    fn default_channel(self) -> BoardCastChannel {
        match self {
            Self::Alt => BoardCastChannel::default_alt_event(),
            Self::Ws => BoardCastChannel::default_ws_event(),
            Self::OrderExecute => BoardCastChannel::default_order_execution(),
            Self::InstIntent => BoardCastChannel::default_inst_intent(),
            Self::ModelPreds => BoardCastChannel::default_model_preds(),
            Self::Schedule => BoardCastChannel::default_scheduler(),
            Self::Trade => BoardCastChannel::default_trade(),
            Self::Lob => BoardCastChannel::default_lob(),
            Self::LobMbo => BoardCastChannel::default_lob_mbo(),
            Self::Candle => BoardCastChannel::default_candle(),
            Self::AccOrder => BoardCastChannel::default_account_order(),
            Self::AccBalPos => BoardCastChannel::default_account_bal_pos(),
            Self::AccPos => BoardCastChannel::default_account_pos(),
        }
    }
}

impl EnvBuilder<HNil> {
    /// Creates an empty runtime builder.
    pub fn new() -> Self {
        Self {
            board_cast_channels: Vec::new(),
            tasks: vec![],
            strategies: HNil,
        }
    }
}

impl Default for EnvBuilder<HNil> {
    fn default() -> Self {
        Self::new()
    }
}

impl<HeadList> EnvBuilder<HeadList> {
    /// Adds an explicit broadcast channel if the same variant is not present.
    ///
    /// Duplicate variants are skipped. For example, adding two trade channels
    /// still leaves one `Trade` broadcast channel in the runtime. An explicit
    /// channel takes precedence over the default inferred from tasks, regardless
    /// of whether it is registered before or after those tasks. A
    /// [`WsChannel::Other`] task infers only its websocket lifecycle channel, so
    /// its custom business stream must be added explicitly.
    pub fn with_board_cast_channel(mut self, channel: BoardCastChannel) -> Self {
        let kind = ChannelKind::of(&channel);
        let channel_type_exists = self
            .board_cast_channels
            .iter()
            .any(|registered| ChannelKind::of(registered) == kind);

        if !channel_type_exists {
            info!("Adding board cast channel: {:?}", channel);
            self.board_cast_channels.push(channel);
        } else {
            info!("Skipped duplicate channel: {:?}", channel);
        }

        self
    }

    /// Adds one runtime task.
    pub fn with_task(mut self, task: TaskInfo) -> Self {
        info!("Adding task: {:?}", task);
        self.tasks.push(task);
        self
    }

    /// Adds several runtime tasks in order.
    pub fn with_tasks(mut self, tasks: Vec<TaskInfo>) -> Self {
        for task in tasks {
            info!("Adding task: {:?}", task);
            self.tasks.push(task);
        }
        self
    }

    /// Registers one strategy module.
    ///
    /// Use this for a single business module. For multiple same-type modules,
    /// use [`EnvBuilder::with_strategy_modules`] so every child gets its own
    /// independent handler loop.
    ///
    /// Calling this method repeatedly creates a static module chain. By default,
    /// modules subscribe to every registered broadcast event for backwards
    /// compatibility. Modules that override `EventHandler::event_mask` subscribe
    /// only to their selected event streams. All modules may independently send
    /// commands to the tasks they care about.
    pub fn with_strategy_module<S>(
        self,
        strategy: S,
    ) -> EnvBuilder<HCons<InnerStrategyModule<S>, HeadList>>
    where
        S: Strategy + Clone,
    {
        info!("Adding strategy: {}", strategy.strategy_name());
        self.with_strategy_node(InnerStrategyModule::new(strategy))
    }

    fn with_strategy_node<N>(self, node: N) -> EnvBuilder<HCons<N, HeadList>>
    where
        N: Strategy + Clone,
    {
        EnvBuilder {
            strategies: HCons {
                head: node,
                tail: self.strategies,
            },
            board_cast_channels: self.board_cast_channels,
            tasks: self.tasks,
        }
    }

    /// Registers many same-type strategy modules.
    ///
    /// The runtime stores the modules in one static HList node, then spawns one
    /// independent event loop per module. This is useful for account-scoped
    /// modules such as per-account order executors.
    ///
    /// This is the public constructor path for strategy groups; the runtime
    /// wrapper itself is intentionally not part of the prelude.
    pub fn with_strategy_modules<S, I>(
        self,
        strategies: I,
    ) -> EnvBuilder<HCons<InnerStrategyGroup<S>, HeadList>>
    where
        S: Strategy + Clone,
        I: IntoIterator<Item = S>,
    {
        let group = InnerStrategyGroup::new(strategies);
        info!("Adding strategy group with {} module(s)", group.len());
        self.with_strategy_node(group)
    }
}

impl<Strategies> EnvBuilder<Strategies>
where
    Strategies: Strategy,
{
    /// Finalizes the builder and fills in channels required by its tasks.
    pub fn build(mut self) -> EnvMediator<Strategies> {
        add_inferred_channels(&mut self.board_cast_channels, &self.tasks);

        EnvMediator {
            core: EnvCore {
                channel: Arc::new(self.board_cast_channels),
                strategy: self.strategies,
            },
            tasks: self.tasks,
        }
    }
}

fn add_inferred_channels(channels: &mut Vec<BoardCastChannel>, tasks: &[TaskInfo]) {
    for kind in inferred_channel_kinds(tasks) {
        if channels
            .iter()
            .any(|registered| ChannelKind::of(registered) == kind)
        {
            continue;
        }

        let channel = kind.default_channel();
        info!("Adding inferred board cast channel: {:?}", channel);
        channels.push(channel);
    }
}

fn inferred_channel_kinds(tasks: &[TaskInfo]) -> Vec<ChannelKind> {
    let mut kinds = Vec::new();

    for task in tasks {
        match task {
            TaskInfo::AltTask(task) => {
                push_unique_kind(&mut kinds, ChannelKind::Alt);
                let primary = match &task.alt_task_type {
                    AltTaskType::OrderExecution => ChannelKind::OrderExecute,
                    AltTaskType::InstIntent => ChannelKind::InstIntent,
                    #[cfg(any(feature = "model_onnx", feature = "model_zmq"))]
                    AltTaskType::ModelPreds(_) => ChannelKind::ModelPreds,
                    AltTaskType::TimeScheduler(_) => ChannelKind::Schedule,
                };
                push_unique_kind(&mut kinds, primary);
            },
            TaskInfo::WsTask(task) => {
                push_unique_kind(&mut kinds, ChannelKind::Ws);
                let primary = match &task.ws_channel {
                    WsChannel::AccountOrders => Some(ChannelKind::AccOrder),
                    WsChannel::AccountBalAndPos => Some(ChannelKind::AccBalPos),
                    WsChannel::AccountPositions => Some(ChannelKind::AccPos),
                    WsChannel::Candles(_) => Some(ChannelKind::Candle),
                    WsChannel::Trades(_) => Some(ChannelKind::Trade),
                    WsChannel::Lob(_) => Some(ChannelKind::Lob),
                    WsChannel::LobMbo => Some(ChannelKind::LobMbo),
                    WsChannel::Other(_) => None,
                };

                if let Some(primary) = primary {
                    push_unique_kind(&mut kinds, primary);
                }
            },
        }
    }

    kinds
}

fn push_unique_kind(kinds: &mut Vec<ChannelKind>, kind: ChannelKind) {
    if !kinds.contains(&kind) {
        kinds.push(kind);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::arch::{
        market_assets::market_core::Market,
        task_execution::{task_alt::AltTaskInfo, task_ws::WsTaskInfo},
    };

    use super::*;

    fn alt_task(alt_task_type: AltTaskType) -> TaskInfo {
        TaskInfo::AltTask(Arc::new(AltTaskInfo {
            alt_task_type,
            chunk: 1,
            task_base_id: None,
        }))
    }

    fn ws_task(ws_channel: WsChannel) -> TaskInfo {
        TaskInfo::WsTask(Arc::new(WsTaskInfo {
            market: Market::HyperLiquid,
            ws_channel,
            filter_channels: false,
            chunk: 1,
            task_base_id: None,
        }))
    }

    fn assert_channel_kinds(channels: &[BoardCastChannel], expected: &[ChannelKind]) {
        assert_eq!(channels.len(), expected.len());
        for expected_kind in expected {
            assert_eq!(
                channels
                    .iter()
                    .filter(|channel| ChannelKind::of(channel) == *expected_kind)
                    .count(),
                1,
                "missing or duplicated channel kind: {expected_kind:?}",
            );
        }
    }

    fn assert_trade_capacity(channels: &[BoardCastChannel], capacity: usize) {
        let sender = channels
            .iter()
            .find_map(|channel| match channel {
                BoardCastChannel::Trade(sender) => Some(sender),
                _ => None,
            })
            .expect("trade channel must exist");
        let mut receiver = sender.subscribe();

        for task_id in 0..=capacity as u64 {
            sender
                .send(
                    crate::arch::strategy_base::handler::handler_core::InfraMsg {
                        task_id,
                        data: Arc::new(Vec::new()),
                    },
                )
                .unwrap();
        }

        assert!(matches!(
            receiver.try_recv(),
            Err(tokio::sync::broadcast::error::TryRecvError::Lagged(1))
        ));
    }

    #[test]
    fn infers_alt_lifecycle_and_primary_channels() {
        let env = EnvBuilder::new()
            .with_tasks(vec![
                alt_task(AltTaskType::OrderExecution),
                alt_task(AltTaskType::InstIntent),
                alt_task(AltTaskType::TimeScheduler(Duration::from_secs(1))),
            ])
            .build();

        assert_channel_kinds(
            &env.core.channel,
            &[
                ChannelKind::Alt,
                ChannelKind::OrderExecute,
                ChannelKind::InstIntent,
                ChannelKind::Schedule,
            ],
        );
    }

    #[cfg(any(feature = "model_onnx", feature = "model_zmq"))]
    #[test]
    fn infers_model_prediction_channel() {
        #[cfg(feature = "model_zmq")]
        let runner = crate::arch::task_execution::task_alt::ModelRunner::Zmq(5_555);
        #[cfg(all(not(feature = "model_zmq"), feature = "model_onnx"))]
        let runner =
            crate::arch::task_execution::task_alt::ModelRunner::Onnx("model.onnx".to_string());

        let env = EnvBuilder::new()
            .with_task(alt_task(AltTaskType::ModelPreds(runner)))
            .build();

        assert_channel_kinds(
            &env.core.channel,
            &[ChannelKind::Alt, ChannelKind::ModelPreds],
        );
    }

    #[test]
    fn infers_ws_lifecycle_and_every_primary_channel() {
        let env = EnvBuilder::new()
            .with_tasks(vec![
                ws_task(WsChannel::AccountOrders),
                ws_task(WsChannel::AccountBalAndPos),
                ws_task(WsChannel::AccountPositions),
                ws_task(WsChannel::Candles(None)),
                ws_task(WsChannel::Trades(None)),
                ws_task(WsChannel::Lob(None)),
                ws_task(WsChannel::LobMbo),
                ws_task(WsChannel::Other("custom".to_string())),
            ])
            .build();

        assert_channel_kinds(
            &env.core.channel,
            &[
                ChannelKind::Ws,
                ChannelKind::AccOrder,
                ChannelKind::AccBalPos,
                ChannelKind::AccPos,
                ChannelKind::Candle,
                ChannelKind::Trade,
                ChannelKind::Lob,
                ChannelKind::LobMbo,
            ],
        );
    }

    #[test]
    fn explicit_capacity_wins_independent_of_call_order() {
        let task = ws_task(WsChannel::Trades(None));
        let explicit_capacity = 32;

        let explicit_before_task = EnvBuilder::new()
            .with_board_cast_channel(BoardCastChannel::trade_with_capacity(explicit_capacity))
            .with_task(task.clone())
            .build();
        let task_before_explicit = EnvBuilder::new()
            .with_task(task)
            .with_board_cast_channel(BoardCastChannel::trade_with_capacity(explicit_capacity))
            .build();

        assert_trade_capacity(&explicit_before_task.core.channel, explicit_capacity);
        assert_trade_capacity(&task_before_explicit.core.channel, explicit_capacity);
        assert_eq!(
            explicit_before_task
                .core
                .channel
                .iter()
                .filter(|channel| matches!(channel, BoardCastChannel::Trade(_)))
                .count(),
            1,
        );
        assert_eq!(
            task_before_explicit
                .core
                .channel
                .iter()
                .filter(|channel| matches!(channel, BoardCastChannel::Trade(_)))
                .count(),
            1,
        );
    }

    #[test]
    fn repeated_tasks_do_not_duplicate_inferred_channels() {
        let env = EnvBuilder::new()
            .with_tasks(vec![
                ws_task(WsChannel::Trades(None)),
                ws_task(WsChannel::Trades(None)),
            ])
            .build();

        assert_channel_kinds(&env.core.channel, &[ChannelKind::Ws, ChannelKind::Trade]);
    }
}
