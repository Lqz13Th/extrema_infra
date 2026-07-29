use std::sync::Arc;

use futures::future;
use tracing::info;

use crate::arch::{
    strategy_base::{
        command::command_core::CommandRegistry,
        handler::{handler_core::strategy_handler_loop, task_channel::TaskChannels},
    },
    task_execution::task_key::TaskKey,
    traits::strategy::{CommandEmitter, EventHandler, Strategy},
};

/// Runtime wrapper for registering many same-type strategy modules.
///
/// `EnvBuilder::with_strategy_modules` stores the modules in this group so the
/// HList runtime can keep one static node while still spawning one handler loop
/// per child module.
#[derive(Clone)]
#[doc(hidden)]
pub struct InnerStrategyGroup<S> {
    strategies: Vec<(S, Option<Arc<[TaskKey]>>)>,
    command_registry: Arc<CommandRegistry>,
}

impl<S> InnerStrategyGroup<S> {
    pub(crate) fn new<I>(strategies: I) -> Self
    where
        I: IntoIterator<Item = (S, Option<Arc<[TaskKey]>>)>,
    {
        Self {
            strategies: strategies.into_iter().collect(),
            command_registry: Arc::new(CommandRegistry::default()),
        }
    }

    pub fn len(&self) -> usize {
        self.strategies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.strategies.is_empty()
    }
}

impl<S> Strategy for InnerStrategyGroup<S>
where
    S: Strategy + Clone + Send + Sync + 'static,
{
    async fn initialize(&mut self) {
        future::join_all(
            self.strategies
                .iter_mut()
                .map(|(strategy, _)| strategy.initialize()),
        )
        .await;
        info!(
            "Initialized strategy group with {} module(s)",
            self.strategies.len()
        );
    }

    fn strategy_name(&self) -> &'static str {
        "InnerStrategyGroup"
    }

    async fn _spawn_strategy_tasks(&self, task_channels: &Arc<TaskChannels>) {
        for (strategy, task_keys) in self.strategies.iter().cloned() {
            let receivers = match task_keys {
                Some(task_keys) => task_channels
                    .subscribe(task_keys.iter().cloned())
                    .expect("strategy task bindings were validated during EnvBuilder::build"),
                None => task_channels.subscribe_all(),
            };

            tokio::spawn(async move {
                info!("Spawned strategy task for {}", strategy.strategy_name());
                strategy_handler_loop(strategy, receivers).await;
            });
        }
    }
}

impl<S> CommandEmitter for InnerStrategyGroup<S>
where
    S: Strategy + Clone + Send + Sync + 'static,
{
    fn command_init(&mut self, registry: Arc<CommandRegistry>) {
        self.command_registry = registry.clone();
        for (strategy, _) in &mut self.strategies {
            strategy.command_init(registry.clone());
        }
    }

    fn command_registry(&self) -> Arc<CommandRegistry> {
        self.command_registry.clone()
    }
}

impl<S> EventHandler for InnerStrategyGroup<S> where S: Strategy + Clone + Send + Sync + 'static {}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use super::*;

    #[derive(Clone)]
    struct ProbeStrategy {
        command_init_count: Arc<AtomicUsize>,
    }

    impl Strategy for ProbeStrategy {
        async fn initialize(&mut self) {}

        fn strategy_name(&self) -> &'static str {
            "ProbeStrategy"
        }
    }

    impl CommandEmitter for ProbeStrategy {
        fn command_init(&mut self, _command_handle: Arc<CommandRegistry>) {
            self.command_init_count.fetch_add(1, Ordering::SeqCst);
        }

        fn command_registry(&self) -> Arc<CommandRegistry> {
            Arc::new(CommandRegistry::default())
        }
    }

    impl EventHandler for ProbeStrategy {}

    #[test]
    fn strategy_group_forwards_command_init_to_children() {
        let command_init_count = Arc::new(AtomicUsize::new(0));
        let mut group = InnerStrategyGroup::new(vec![
            (
                ProbeStrategy {
                    command_init_count: command_init_count.clone(),
                },
                None,
            ),
            (
                ProbeStrategy {
                    command_init_count: command_init_count.clone(),
                },
                None,
            ),
        ]);

        group.command_init(Arc::new(CommandRegistry::default()));

        assert_eq!(command_init_count.load(Ordering::SeqCst), 2);
    }
}
