use std::sync::Arc;
use tracing::info;

use crate::{
    arch::{
        infra_core::{env_core::EnvCore, env_mediator::EnvMediator},
        strategy_base::{
            handler::task_channel::TaskChannels,
            hlist_core::{HCons, HNil},
            strategy_group::InnerStrategyGroup,
            strategy_module::InnerStrategyModule,
        },
        task_execution::{task_general::TaskInfo, task_key::TaskKey},
        traits::strategy::Strategy,
    },
    errors::{InfraError, InfraResult},
};

/// Builder for an `extrema_infra` runtime.
///
/// Use this builder in the final binary to declare runtime tasks and strategy
/// modules. Every concrete task owns one broadcast stream. Strategies receive
/// all task streams by default and can opt into an explicit set of [`TaskKey`]
/// values with [`EnvBuilder::with_strategy_module_on`].
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
///     .with_task(task)
///     .with_strategy_module(my_strategy)
///     .build()
///     .expect("invalid runtime configuration");
/// ```
pub struct EnvBuilder<Strategies = HNil> {
    tasks: Vec<TaskInfo>,
    strategies: Strategies,
    explicit_bindings: Vec<Arc<[TaskKey]>>,
}

impl EnvBuilder<HNil> {
    /// Creates an empty runtime builder.
    pub fn new() -> Self {
        Self {
            tasks: vec![],
            strategies: HNil,
            explicit_bindings: Vec::new(),
        }
    }
}

impl Default for EnvBuilder<HNil> {
    fn default() -> Self {
        Self::new()
    }
}

impl<HeadList> EnvBuilder<HeadList> {
    /// Adds one runtime task.
    pub fn with_task(mut self, task: impl Into<TaskInfo>) -> Self {
        let task = task.into();
        info!("Adding task: {:?}", task);
        self.tasks.push(task);
        self
    }

    /// Adds several runtime tasks in order.
    pub fn with_tasks<T>(mut self, tasks: impl IntoIterator<Item = T>) -> Self
    where
        T: Into<TaskInfo>,
    {
        for task in tasks {
            let task = task.into();
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
    /// Calling this method repeatedly creates a static module chain. Every
    /// module registered through this method receives every task stream.
    pub fn with_strategy_module<S>(
        self,
        strategy: S,
    ) -> EnvBuilder<HCons<InnerStrategyModule<S>, HeadList>>
    where
        S: Strategy + Clone,
    {
        info!("Adding strategy: {}", strategy.strategy_name());
        self.with_strategy_node(InnerStrategyModule::new(strategy), None)
    }

    /// Registers one strategy module for an explicit set of concrete tasks.
    pub fn with_strategy_module_on<S, I>(
        self,
        strategy: S,
        task_keys: I,
    ) -> EnvBuilder<HCons<InnerStrategyModule<S>, HeadList>>
    where
        S: Strategy + Clone,
        I: IntoIterator<Item = TaskKey>,
    {
        let task_keys: Arc<[TaskKey]> = task_keys.into_iter().collect::<Vec<_>>().into();
        info!(
            "Adding strategy {} on {} task(s)",
            strategy.strategy_name(),
            task_keys.len()
        );
        self.with_strategy_node(
            InnerStrategyModule::on(strategy, task_keys.clone()),
            Some(task_keys),
        )
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
        let group =
            InnerStrategyGroup::new(strategies.into_iter().map(|strategy| (strategy, None)));
        info!("Adding strategy group with {} module(s)", group.len());
        self.with_strategy_node(group, None)
    }

    /// Registers many same-type strategy modules with independent task sets.
    ///
    /// Each `(strategy, task_keys)` pair gets its own handler loop and subscribes
    /// only to those concrete tasks. Use repeated
    /// [`EnvBuilder::with_strategy_module_on`] calls when the modules have
    /// different concrete Rust types.
    pub fn with_strategy_modules_on<S, I>(
        mut self,
        strategies: I,
    ) -> EnvBuilder<HCons<InnerStrategyGroup<S>, HeadList>>
    where
        S: Strategy + Clone,
        I: IntoIterator<Item = (S, Vec<TaskKey>)>,
    {
        let mut modules = Vec::new();
        for (strategy, task_keys) in strategies {
            let task_keys: Arc<[TaskKey]> = task_keys.into();
            self.explicit_bindings.push(task_keys.clone());
            modules.push((strategy, Some(task_keys)));
        }

        let group = InnerStrategyGroup::new(modules);
        info!("Adding bound strategy group with {} module(s)", group.len());
        self.with_strategy_node(group, None)
    }

    fn with_strategy_node<N>(
        mut self,
        node: N,
        explicit_binding: Option<Arc<[TaskKey]>>,
    ) -> EnvBuilder<HCons<N, HeadList>>
    where
        N: Strategy + Clone,
    {
        if let Some(binding) = explicit_binding {
            self.explicit_bindings.push(binding);
        }

        EnvBuilder {
            strategies: HCons {
                head: node,
                tail: self.strategies,
            },
            tasks: self.tasks,
            explicit_bindings: self.explicit_bindings,
        }
    }
}

impl<Strategies> EnvBuilder<Strategies>
where
    Strategies: Strategy,
{
    /// Validates task bindings and creates one broadcast stream per task.
    pub fn build(self) -> InfraResult<EnvMediator<Strategies>> {
        let mut task_keys = Vec::new();
        for task in &self.tasks {
            task_keys.extend(task.task_keys()?);
        }

        let task_channels = TaskChannels::new(task_keys)?;

        for binding in &self.explicit_bindings {
            for task_key in binding.iter() {
                if !task_channels.contains(task_key) {
                    return Err(InfraError::Msg(format!(
                        "strategy references an unregistered task: {task_key:?}"
                    )));
                }
            }
        }

        Ok(EnvMediator {
            core: EnvCore {
                task_channels: Arc::new(task_channels),
                strategy: self.strategies,
            },
            tasks: self.tasks,
        })
    }
}

#[cfg(test)]
mod task_channel_tests {
    use std::time::Duration;

    use crate::arch::{
        market_assets::market_core::Market,
        task_execution::{
            task_alt::{AltTaskInfo, AltTaskType},
            task_general::TaskInfo,
            task_key::TaskKey,
            task_ws::{TradesParam, WsChannel, WsTaskInfo},
        },
    };

    use super::*;

    fn trade_task(market: Market, task_id: u64) -> TaskInfo {
        ws_task(market, WsChannel::Trades(None), task_id)
    }

    fn ws_task(market: Market, ws_channel: WsChannel, task_id: u64) -> TaskInfo {
        TaskInfo::WsTask(Arc::new(WsTaskInfo {
            market,
            ws_channel,
            filter_channels: false,
            chunk: 1,
            task_base_id: Some(task_id),
        }))
    }

    fn scheduler_task(duration: Duration, task_id: u64) -> TaskInfo {
        TaskInfo::AltTask(Arc::new(AltTaskInfo {
            alt_task_type: AltTaskType::TimeScheduler(duration),
            chunk: 1,
            task_base_id: Some(task_id),
        }))
    }

    #[test]
    fn creates_one_channel_per_concrete_task() {
        let task = trade_task(Market::BinanceUmFutures, 7);
        let key = task.task_key(7);
        let env = EnvBuilder::new().with_task(task).build().unwrap();

        assert!(env.core.task_channels.contains(&key));
    }

    #[test]
    fn task_descriptors_register_without_task_info_wrappers() {
        let scheduler = AltTaskInfo {
            alt_task_type: AltTaskType::TimeScheduler(Duration::from_secs(1)),
            chunk: 1,
            task_base_id: Some(11),
        };
        let trades = WsTaskInfo {
            market: Market::BinanceUmFutures,
            ws_channel: WsChannel::Trades(None),
            filter_channels: false,
            chunk: 1,
            task_base_id: Some(12),
        };
        let scheduler_key = TaskKey::alt(&scheduler.alt_task_type, 11);
        let trades_key = TaskKey::ws(&trades.ws_channel, 12);

        let env = EnvBuilder::new()
            .with_task(scheduler)
            .with_tasks([trades])
            .build()
            .unwrap();

        assert!(env.core.task_channels.contains(&scheduler_key));
        assert!(env.core.task_channels.contains(&trades_key));
        assert_eq!(env.tasks().len(), 2);
    }

    #[test]
    fn same_ws_channel_and_id_is_rejected_across_markets() {
        let error = EnvBuilder::new()
            .with_tasks(vec![
                trade_task(Market::BinanceUmFutures, 7),
                trade_task(Market::Okx, 7),
            ])
            .build()
            .err()
            .expect("market is not part of a websocket task key");

        assert!(
            error
                .to_string()
                .contains("duplicate task id for the same task type")
        );
    }

    #[test]
    fn same_trade_callback_id_is_rejected_across_stream_parameters() {
        let error = EnvBuilder::new()
            .with_tasks(vec![
                ws_task(
                    Market::BinanceUmFutures,
                    WsChannel::Trades(Some(TradesParam::AggTrades)),
                    7,
                ),
                ws_task(
                    Market::BinanceUmFutures,
                    WsChannel::Trades(Some(TradesParam::AllTrades)),
                    7,
                ),
            ])
            .build()
            .err()
            .expect("trade stream parameters share one callback id namespace");

        assert!(
            error
                .to_string()
                .contains("duplicate task id for the same task type")
        );
    }

    #[test]
    fn same_schedule_callback_id_is_rejected_across_durations() {
        let error = EnvBuilder::new()
            .with_tasks(vec![
                scheduler_task(Duration::from_secs(1), 7),
                scheduler_task(Duration::from_secs(5), 7),
            ])
            .build()
            .err()
            .expect("scheduler durations share one callback id namespace");

        assert!(
            error
                .to_string()
                .contains("duplicate task id for the same task type")
        );
    }

    #[test]
    fn different_task_channels_may_reuse_task_id() {
        EnvBuilder::new()
            .with_tasks(vec![
                ws_task(Market::BinanceUmFutures, WsChannel::Trades(None), 7),
                ws_task(Market::BinanceUmFutures, WsChannel::Lob(None), 7),
            ])
            .build()
            .expect("trade and lob channels have distinct task keys");
    }

    #[test]
    fn explicit_binding_must_reference_a_registered_task() {
        let missing = TaskKey::ws(&WsChannel::Trades(None), 99);
        let error = EnvBuilder::new()
            .with_strategy_module_on(HNil, [missing])
            .build()
            .err()
            .expect("unknown task binding must fail");

        assert!(error.to_string().contains("unregistered task"));
    }

    #[test]
    fn overflowing_task_id_range_is_a_build_error() {
        let task = TaskInfo::WsTask(Arc::new(WsTaskInfo {
            market: Market::BinanceUmFutures,
            ws_channel: WsChannel::Trades(None),
            filter_channels: false,
            chunk: 2,
            task_base_id: Some(u64::MAX),
        }));

        let error = EnvBuilder::new()
            .with_task(task)
            .build()
            .err()
            .expect("overflowing task ids must fail");

        assert!(error.to_string().contains("overflows u64"));
    }
}
