use std::{collections::HashSet, sync::Arc, time::Duration};

use extrema_infra::prelude::*;
use tokio::sync::mpsc;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);
const ISOLATION_WINDOW: Duration = Duration::from_millis(100);

#[derive(Clone)]
struct LifecycleProbe {
    events: mpsc::UnboundedSender<TaskKey>,
    registry: Arc<CommandRegistry>,
}

impl LifecycleProbe {
    fn new() -> (Self, mpsc::UnboundedReceiver<TaskKey>) {
        let (events, receiver) = mpsc::unbounded_channel();
        (
            Self {
                events,
                registry: Arc::new(CommandRegistry::default()),
            },
            receiver,
        )
    }
}

impl Strategy for LifecycleProbe {
    async fn initialize(&mut self) {}

    fn strategy_name(&self) -> &'static str {
        "LifecycleProbe"
    }
}

impl CommandEmitter for LifecycleProbe {
    fn command_init(&mut self, registry: Arc<CommandRegistry>) {
        self.registry = registry;
    }

    fn command_registry(&self) -> Arc<CommandRegistry> {
        self.registry.clone()
    }
}

impl EventHandler for LifecycleProbe {
    async fn on_ws_event(&mut self, msg: InfraMsg<WsTaskInfo>) {
        let key = TaskKey::ws(&msg.data.ws_channel, msg.task_id);
        let _ = self.events.send(key);
    }
}

fn ws_task(ws_channel: WsChannel, chunk: u64) -> TaskInfo {
    TaskInfo::WsTask(Arc::new(WsTaskInfo {
        market: Market::HyperLiquid,
        ws_channel,
        filter_channels: false,
        chunk,
        task_base_id: Some(1),
    }))
}

fn trade_key(task_id: u64) -> TaskKey {
    TaskKey::ws(&WsChannel::Trades(None), task_id)
}

fn lob_key(task_id: u64) -> TaskKey {
    TaskKey::ws(&WsChannel::Lob(None), task_id)
}

async fn receive_keys(
    receiver: &mut mpsc::UnboundedReceiver<TaskKey>,
    count: usize,
) -> HashSet<TaskKey> {
    tokio::time::timeout(STARTUP_TIMEOUT, async {
        let mut keys = HashSet::with_capacity(count);
        while keys.len() < count {
            let key = receiver
                .recv()
                .await
                .expect("strategy lifecycle event channel closed");
            assert!(keys.insert(key), "received a duplicate lifecycle event");
        }
        keys
    })
    .await
    .expect("strategy did not receive the expected task lifecycle events")
}

async fn assert_no_more_events(receiver: &mut mpsc::UnboundedReceiver<TaskKey>) {
    assert!(
        tokio::time::timeout(ISOLATION_WINDOW, receiver.recv())
            .await
            .is_err(),
        "strategy received an event from an unbound task"
    );
}

#[tokio::test]
async fn one_task_fans_out_to_multiple_bound_strategies() {
    let trade_1 = trade_key(1);
    let (first, mut first_events) = LifecycleProbe::new();
    let (second, mut second_events) = LifecycleProbe::new();
    let env = EnvBuilder::new()
        .with_task(ws_task(WsChannel::Trades(None), 1))
        .with_strategy_modules_on([
            (first, vec![trade_1.clone()]),
            (second, vec![trade_1.clone()]),
        ])
        .build()
        .unwrap();

    let runtime = tokio::spawn(env.execute());

    assert_eq!(
        receive_keys(&mut first_events, 1).await,
        HashSet::from([trade_1.clone()])
    );
    assert_eq!(
        receive_keys(&mut second_events, 1).await,
        HashSet::from([trade_1])
    );

    runtime.abort();
}

#[tokio::test]
async fn same_type_strategy_group_defaults_to_all_tasks() {
    let trade_1 = trade_key(1);
    let (first, mut first_events) = LifecycleProbe::new();
    let (second, mut second_events) = LifecycleProbe::new();
    let env = EnvBuilder::new()
        .with_task(ws_task(WsChannel::Trades(None), 1))
        .with_strategy_modules([first, second])
        .build()
        .unwrap();

    let runtime = tokio::spawn(env.execute());

    assert_eq!(
        receive_keys(&mut first_events, 1).await,
        HashSet::from([trade_1.clone()])
    );
    assert_eq!(
        receive_keys(&mut second_events, 1).await,
        HashSet::from([trade_1])
    );

    runtime.abort();
}

#[tokio::test]
async fn explicit_bindings_isolate_strategy_task_streams() {
    let trade_1 = trade_key(1);
    let trade_2 = trade_key(2);
    let (first, mut first_events) = LifecycleProbe::new();
    let (second, mut second_events) = LifecycleProbe::new();
    let env = EnvBuilder::new()
        .with_task(ws_task(WsChannel::Trades(None), 2))
        .with_strategy_modules_on([
            (first, vec![trade_1.clone()]),
            (second, vec![trade_2.clone()]),
        ])
        .build()
        .unwrap();

    let runtime = tokio::spawn(env.execute());

    assert_eq!(
        receive_keys(&mut first_events, 1).await,
        HashSet::from([trade_1])
    );
    assert_eq!(
        receive_keys(&mut second_events, 1).await,
        HashSet::from([trade_2])
    );
    assert_no_more_events(&mut first_events).await;
    assert_no_more_events(&mut second_events).await;

    runtime.abort();
}

#[tokio::test]
async fn strategy_can_bind_trade_tasks_one_and_three() {
    let trade_1 = trade_key(1);
    let trade_3 = trade_key(3);
    let (strategy, mut events) = LifecycleProbe::new();
    let env = EnvBuilder::new()
        .with_task(ws_task(WsChannel::Trades(None), 3))
        .with_strategy_module_on(strategy, [trade_1.clone(), trade_3.clone()])
        .build()
        .unwrap();

    let runtime = tokio::spawn(env.execute());

    assert_eq!(
        receive_keys(&mut events, 2).await,
        HashSet::from([trade_1, trade_3])
    );
    assert_no_more_events(&mut events).await;

    runtime.abort();
}

#[tokio::test]
async fn strategy_can_bind_trade_one_and_lob_one() {
    let trade_1 = trade_key(1);
    let lob_1 = lob_key(1);
    let (strategy, mut events) = LifecycleProbe::new();
    let env = EnvBuilder::new()
        .with_tasks(vec![
            ws_task(WsChannel::Trades(None), 1),
            ws_task(WsChannel::Lob(None), 1),
        ])
        .with_strategy_module_on(strategy, [trade_1.clone(), lob_1.clone()])
        .build()
        .unwrap();

    let runtime = tokio::spawn(env.execute());

    assert_eq!(
        receive_keys(&mut events, 2).await,
        HashSet::from([trade_1, lob_1])
    );

    runtime.abort();
}

#[tokio::test]
async fn strategy_without_binding_receives_all_task_streams() {
    let expected = HashSet::from([trade_key(1), trade_key(2), trade_key(3), lob_key(1)]);
    let (strategy, mut events) = LifecycleProbe::new();
    let env = EnvBuilder::new()
        .with_tasks(vec![
            ws_task(WsChannel::Trades(None), 3),
            ws_task(WsChannel::Lob(None), 1),
        ])
        .with_strategy_module(strategy)
        .build()
        .unwrap();

    let runtime = tokio::spawn(env.execute());

    assert_eq!(receive_keys(&mut events, expected.len()).await, expected);

    runtime.abort();
}

#[tokio::test]
async fn startup_lifecycle_event_is_not_lost() {
    let trade_1 = trade_key(1);
    let (strategy, mut events) = LifecycleProbe::new();
    let env = EnvBuilder::new()
        .with_task(ws_task(WsChannel::Trades(None), 1))
        .with_strategy_module_on(strategy, [trade_1.clone()])
        .build()
        .unwrap();

    let runtime = tokio::spawn(env.execute());

    assert_eq!(receive_keys(&mut events, 1).await, HashSet::from([trade_1]));

    runtime.abort();
}
