use std::sync::Arc;

use tracing::info;

use crate::arch::{
    strategy_base::{
        command::command_core::CommandRegistry,
        handler::{handler_core::strategy_handler_loop, task_channel::TaskChannels},
    },
    task_execution::TaskKey,
    traits::strategy::{CommandEmitter, EventHandler, Strategy},
};

/// Runtime wrapper for one ordinary strategy module.
///
/// The wrapped strategy owns the business callbacks. This node owns the runtime
/// spawn boundary used by the HList environment.
#[derive(Clone)]
#[doc(hidden)]
pub struct InnerStrategyModule<S> {
    strategy: S,
    task_keys: Option<Arc<[TaskKey]>>,
}

impl<S> InnerStrategyModule<S> {
    pub(crate) fn new(strategy: S) -> Self {
        Self {
            strategy,
            task_keys: None,
        }
    }

    pub(crate) fn on(strategy: S, task_keys: Arc<[TaskKey]>) -> Self {
        Self {
            strategy,
            task_keys: Some(task_keys),
        }
    }
}

impl<S> Strategy for InnerStrategyModule<S>
where
    S: Strategy + Clone + Send + Sync + 'static,
{
    async fn initialize(&mut self) {
        self.strategy.initialize().await;
    }

    fn strategy_name(&self) -> &'static str {
        self.strategy.strategy_name()
    }

    async fn _spawn_strategy_tasks(&self, task_channels: &Arc<TaskChannels>) {
        let receivers = match &self.task_keys {
            Some(task_keys) => task_channels
                .subscribe(task_keys.iter().cloned())
                .expect("strategy task bindings were validated during EnvBuilder::build"),
            None => task_channels.subscribe_all(),
        };
        let strategy = self.strategy.clone();

        tokio::spawn(async move {
            info!("Spawned strategy task for {}", strategy.strategy_name());
            strategy_handler_loop(strategy, receivers).await;
        });
    }
}

impl<S> CommandEmitter for InnerStrategyModule<S>
where
    S: Strategy + Clone + Send + Sync + 'static,
{
    fn command_init(&mut self, registry: Arc<CommandRegistry>) {
        self.strategy.command_init(registry);
    }

    fn command_registry(&self) -> Arc<CommandRegistry> {
        self.strategy.command_registry()
    }
}

impl<S> EventHandler for InnerStrategyModule<S> where S: Strategy + Clone + Send + Sync + 'static {}

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
    fn strategy_module_forwards_command_init() {
        let command_init_count = Arc::new(AtomicUsize::new(0));
        let mut module = InnerStrategyModule::new(ProbeStrategy {
            command_init_count: command_init_count.clone(),
        });

        module.command_init(Arc::new(CommandRegistry::default()));

        assert_eq!(command_init_count.load(Ordering::SeqCst), 1);
        assert!(module.task_keys.is_none());
    }
}
