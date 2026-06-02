use std::sync::Arc;

use tracing::info;

use crate::arch::{
    strategy_base::{
        command::command_core::CommandRegistry,
        handler::handler_core::{BoardCastChannel, strategy_handler_loop},
    },
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
}

impl<S> InnerStrategyModule<S> {
    pub(crate) fn new(strategy: S) -> Self {
        Self { strategy }
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

    async fn _spawn_strategy_tasks(&self, channels: &Arc<Vec<BoardCastChannel>>) {
        let channels = channels.clone();
        let strategy = self.strategy.clone();

        tokio::spawn(async move {
            info!("Spawned strategy task for {}", strategy.strategy_name());
            strategy_handler_loop(strategy, &channels).await;
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

    use crate::arch::strategy_base::handler::event_mask::EventMask;

    use super::*;

    #[derive(Clone)]
    struct ProbeStrategy {
        command_init_count: Arc<AtomicUsize>,
        loop_start_count: Arc<AtomicUsize>,
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

    impl EventHandler for ProbeStrategy {
        fn event_mask(&self) -> EventMask {
            self.loop_start_count.fetch_add(1, Ordering::SeqCst);
            EventMask::NONE
        }
    }

    async fn wait_for_count(counter: &AtomicUsize, expected: usize) {
        for _ in 0..10 {
            if counter.load(Ordering::SeqCst) >= expected {
                return;
            }
            tokio::task::yield_now().await;
        }
    }

    #[tokio::test]
    async fn strategy_module_spawns_wrapped_strategy_once() {
        let loop_start_count = Arc::new(AtomicUsize::new(0));
        let module = InnerStrategyModule::new(ProbeStrategy {
            command_init_count: Arc::new(AtomicUsize::new(0)),
            loop_start_count: loop_start_count.clone(),
        });

        module._spawn_strategy_tasks(&Arc::new(vec![])).await;
        wait_for_count(&loop_start_count, 1).await;

        assert_eq!(loop_start_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn strategy_module_forwards_command_init() {
        let command_init_count = Arc::new(AtomicUsize::new(0));
        let mut module = InnerStrategyModule::new(ProbeStrategy {
            command_init_count: command_init_count.clone(),
            loop_start_count: Arc::new(AtomicUsize::new(0)),
        });

        module.command_init(Arc::new(CommandRegistry::default()));

        assert_eq!(command_init_count.load(Ordering::SeqCst), 1);
    }
}
