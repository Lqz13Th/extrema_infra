use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use extrema_infra::prelude::*;

#[derive(Clone)]
struct ProbeStrategy {
    initialize_count: Arc<AtomicUsize>,
    command_init_count: Arc<AtomicUsize>,
    event_mask_count: Arc<AtomicUsize>,
    registry: Arc<CommandRegistry>,
}

impl ProbeStrategy {
    fn new(
        initialize_count: Arc<AtomicUsize>,
        command_init_count: Arc<AtomicUsize>,
        event_mask_count: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            initialize_count,
            command_init_count,
            event_mask_count,
            registry: Arc::new(CommandRegistry::default()),
        }
    }
}

impl Strategy for ProbeStrategy {
    async fn initialize(&mut self) {
        self.initialize_count.fetch_add(1, Ordering::SeqCst);
    }

    fn strategy_name(&self) -> &'static str {
        "ProbeStrategy"
    }
}

impl CommandEmitter for ProbeStrategy {
    fn command_init(&mut self, registry: Arc<CommandRegistry>) {
        self.command_init_count.fetch_add(1, Ordering::SeqCst);
        self.registry = registry;
    }

    fn command_registry(&self) -> Arc<CommandRegistry> {
        self.registry.clone()
    }
}

impl EventHandler for ProbeStrategy {
    fn event_mask(&self) -> EventMask {
        self.event_mask_count.fetch_add(1, Ordering::SeqCst);
        EventMask::NONE
    }
}

async fn wait_for_counter(counter: Arc<AtomicUsize>, expected: usize) {
    tokio::time::timeout(Duration::from_secs(1), async move {
        loop {
            if counter.load(Ordering::SeqCst) >= expected {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("strategy runtime did not reach expected count");
}

#[tokio::test]
async fn env_builder_starts_single_strategy_handler_loop() {
    let initialize_count = Arc::new(AtomicUsize::new(0));
    let command_init_count = Arc::new(AtomicUsize::new(0));
    let event_mask_count = Arc::new(AtomicUsize::new(0));

    let env = EnvBuilder::new()
        .with_strategy_module(ProbeStrategy::new(
            initialize_count.clone(),
            command_init_count.clone(),
            event_mask_count.clone(),
        ))
        .build();

    let runtime = tokio::spawn(env.execute());
    wait_for_counter(event_mask_count.clone(), 1).await;
    runtime.abort();

    assert_eq!(initialize_count.load(Ordering::SeqCst), 1);
    assert_eq!(command_init_count.load(Ordering::SeqCst), 1);
    assert_eq!(event_mask_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn env_builder_starts_repeated_single_strategy_handler_loops() {
    let initialize_count = Arc::new(AtomicUsize::new(0));
    let command_init_count = Arc::new(AtomicUsize::new(0));
    let event_mask_count = Arc::new(AtomicUsize::new(0));

    let env = EnvBuilder::new()
        .with_strategy_module(ProbeStrategy::new(
            initialize_count.clone(),
            command_init_count.clone(),
            event_mask_count.clone(),
        ))
        .with_strategy_module(ProbeStrategy::new(
            initialize_count.clone(),
            command_init_count.clone(),
            event_mask_count.clone(),
        ))
        .build();

    let runtime = tokio::spawn(env.execute());
    wait_for_counter(event_mask_count.clone(), 2).await;
    runtime.abort();

    assert_eq!(initialize_count.load(Ordering::SeqCst), 2);
    assert_eq!(command_init_count.load(Ordering::SeqCst), 2);
    assert_eq!(event_mask_count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn env_builder_starts_many_same_type_strategy_handler_loops() {
    let initialize_count = Arc::new(AtomicUsize::new(0));
    let command_init_count = Arc::new(AtomicUsize::new(0));
    let event_mask_count = Arc::new(AtomicUsize::new(0));

    let modules = (0..3)
        .map(|_| {
            ProbeStrategy::new(
                initialize_count.clone(),
                command_init_count.clone(),
                event_mask_count.clone(),
            )
        })
        .collect::<Vec<_>>();

    let env = EnvBuilder::new().with_strategy_modules(modules).build();

    let runtime = tokio::spawn(env.execute());
    wait_for_counter(event_mask_count.clone(), 3).await;
    runtime.abort();

    assert_eq!(initialize_count.load(Ordering::SeqCst), 3);
    assert_eq!(command_init_count.load(Ordering::SeqCst), 3);
    assert_eq!(event_mask_count.load(Ordering::SeqCst), 3);
}
