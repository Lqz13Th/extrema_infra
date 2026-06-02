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
    task_execution::task_general::TaskInfo,
    traits::strategy::Strategy,
};

/// Builder for an `extrema_infra` runtime.
///
/// Use this builder in the final binary to declare broadcast channels, runtime
/// tasks, and strategy modules. `with_strategy_module` keeps each strategy as
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
///     .with_board_cast_channel(BoardCastChannel::default_alt_event())
///     .with_board_cast_channel(BoardCastChannel::default_scheduler())
///     .with_task(TaskInfo::AltTask(Arc::new(task)))
///     .with_strategy_module(my_strategy)
///     .build();
/// ```
pub struct EnvBuilder<Strategies = HNil> {
    board_cast_channels: Vec<BoardCastChannel>,
    tasks: Vec<TaskInfo>,
    strategies: Strategies,
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
    /// Adds a broadcast channel if a channel of the same variant is not already
    /// present.
    ///
    /// Duplicate variants are skipped. For example, adding two trade channels
    /// still leaves one `Trade` broadcast channel in the runtime.
    pub fn with_board_cast_channel(mut self, channel: BoardCastChannel) -> Self {
        let channel_type_exists = self
            .board_cast_channels
            .iter()
            .any(|ch| std::mem::discriminant(ch) == std::mem::discriminant(&channel));

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
    /// Finalizes the builder into an executable environment.
    pub fn build(self) -> EnvMediator<Strategies> {
        EnvMediator {
            core: EnvCore {
                channel: Arc::new(self.board_cast_channels),
                strategy: self.strategies,
            },
            tasks: self.tasks,
        }
    }
}
