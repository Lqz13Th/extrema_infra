use std::{
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use futures::{Stream, StreamExt};
use tokio_stream::wrappers::{BroadcastStream, errors::BroadcastStreamRecvError};
use tracing::error;

use crate::arch::{
    strategy_base::handler::task_channel::{TaskEvent, TaskReceiver},
    task_execution::TaskKey,
    traits::strategy::Strategy,
};

const LAG_REPORT_INTERVAL: Duration = Duration::from_secs(1);

fn record_lag(
    last_report_at: &mut Option<Instant>,
    skipped_since_report: &mut u64,
    skipped: u64,
) -> Option<u64> {
    *skipped_since_report = skipped_since_report.saturating_add(skipped);
    let now = Instant::now();
    let should_report = last_report_at
        .is_none_or(|last_report| now.duration_since(last_report) >= LAG_REPORT_INTERVAL);

    should_report.then(|| {
        *last_report_at = Some(now);
        std::mem::take(skipped_since_report)
    })
}

async fn dispatch_task_event<S>(strategy: &mut S, event: TaskEvent)
where
    S: Strategy,
{
    match event {
        TaskEvent::Alt(msg) => strategy.on_alt_event(msg).await,
        TaskEvent::Ws(msg) => strategy.on_ws_event(msg).await,
        TaskEvent::OrderExecute(msg) => strategy.on_order_execution(msg).await,
        TaskEvent::InstIntent(msg) => strategy.on_inst_intent(msg).await,
        TaskEvent::ModelPreds(msg) => strategy.on_preds(msg).await,
        TaskEvent::Schedule(msg) => strategy.on_schedule(msg).await,
        TaskEvent::Trade(msg) => strategy.on_trade(msg).await,
        TaskEvent::Lob(msg) => strategy.on_lob(msg).await,
        TaskEvent::LobMbo(msg) => strategy.on_lob_mbo(msg).await,
        TaskEvent::Candle(msg) => strategy.on_candle(msg).await,
        TaskEvent::AccOrder(msg) => strategy.on_acc_order(msg).await,
        TaskEvent::AccBalPos(msg) => strategy.on_acc_bal_pos(msg).await,
        TaskEvent::AccPos(msg) => strategy.on_acc_pos(msg).await,
        TaskEvent::WsOther(msg) => strategy.on_ws_other(msg).await,
    }
}

pub(crate) async fn strategy_handler_loop<S>(mut strategy: S, receivers: Vec<TaskReceiver>)
where
    S: Strategy,
{
    let mut events = TaskEventMux::new(receivers);
    while let Some(item) = events.next().await {
        match item {
            MuxItem::Event(event) => dispatch_task_event(&mut strategy, event).await,
            MuxItem::Lagged { key, skipped } => {
                error!(task_key = ?key, skipped, "task event receiver lagged");
            },
            MuxItem::LaggedSuppressed => {},
        }
    }
}

struct TaskStream {
    key: TaskKey,
    stream: BroadcastStream<TaskEvent>,
    last_lag_report_at: Option<Instant>,
    skipped_since_report: u64,
    closed: bool,
}

enum MuxItem {
    Event(TaskEvent),
    Lagged { key: TaskKey, skipped: u64 },
    LaggedSuppressed,
}

/// Allocation-stable adaptive fan-in for a strategy's selected task streams.
///
/// One rotating non-hot stream is probed before the last-ready stream. This
/// keeps a single hot stream cheap while guaranteeing that another ready stream
/// is observed within at most `N - 1` hot events. Before returning `Pending`,
/// every open stream is polled so every receiver registers the outer waker.
struct TaskEventMux {
    streams: Vec<TaskStream>,
    open: usize,
    hot: Option<usize>,
    probe_cursor: usize,
}

impl TaskEventMux {
    fn new(receivers: Vec<TaskReceiver>) -> Self {
        let streams = receivers
            .into_iter()
            .map(|receiver| TaskStream {
                key: receiver.key,
                stream: BroadcastStream::new(receiver.receiver),
                last_lag_report_at: None,
                skipped_since_report: 0,
                closed: false,
            })
            .collect::<Vec<_>>();
        let open = streams.len();

        Self {
            streams,
            open,
            hot: (open > 0).then_some(0),
            probe_cursor: usize::from(open > 1),
        }
    }

    fn poll_lane(&mut self, index: usize, cx: &mut Context<'_>) -> Option<MuxItem> {
        debug_assert!(!self.streams[index].closed);

        let item = match Pin::new(&mut self.streams[index].stream).poll_next(cx) {
            Poll::Ready(Some(item)) => item,
            Poll::Ready(None) => {
                self.streams[index].closed = true;
                self.open -= 1;
                if self.hot == Some(index) {
                    self.hot = None;
                }
                return None;
            },
            Poll::Pending => return None,
        };

        self.hot = Some(index);
        Some(self.ready_item(index, item))
    }

    fn next_probe(&mut self, hot: Option<usize>) -> Option<usize> {
        let len = self.streams.len();
        for offset in 0..len {
            let index = (self.probe_cursor + offset) % len;
            if !self.streams[index].closed && Some(index) != hot {
                self.probe_cursor = (index + 1) % len;
                return Some(index);
            }
        }
        None
    }

    fn ready_item(
        &mut self,
        index: usize,
        item: Result<TaskEvent, BroadcastStreamRecvError>,
    ) -> MuxItem {
        match item {
            Ok(event) => MuxItem::Event(event),
            Err(BroadcastStreamRecvError::Lagged(skipped)) => {
                let stream = &mut self.streams[index];
                if let Some(skipped) = record_lag(
                    &mut stream.last_lag_report_at,
                    &mut stream.skipped_since_report,
                    skipped,
                ) {
                    MuxItem::Lagged {
                        key: stream.key.clone(),
                        skipped,
                    }
                } else {
                    MuxItem::LaggedSuppressed
                }
            },
        }
    }
}

impl Stream for TaskEventMux {
    type Item = MuxItem;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.open == 0 {
            return Poll::Ready(None);
        }

        let preferred_hot = self.hot;
        let probe = self.next_probe(preferred_hot);

        if let Some(index) = probe
            && let Some(item) = self.poll_lane(index, cx)
        {
            return Poll::Ready(Some(item));
        }

        if let Some(index) = preferred_hot
            && let Some(item) = self.poll_lane(index, cx)
        {
            return Poll::Ready(Some(item));
        }

        for index in 0..self.streams.len() {
            if self.streams[index].closed || Some(index) == probe || Some(index) == preferred_hot {
                continue;
            }

            if let Some(item) = self.poll_lane(index, cx) {
                return Poll::Ready(Some(item));
            }
        }

        if self.open == 0 {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, sync::Arc, time::Duration};

    use tokio::sync::{broadcast, mpsc};

    use crate::arch::{
        strategy_base::{
            command::command_core::CommandRegistry,
            handler::{
                alt_events::AltScheduleEvent,
                lob_events::{WsLob, WsTrade},
                task_channel::{InfraMsg, TaskChannels, TaskEvent, TaskReceiver},
            },
        },
        task_execution::{TaskKey, task_alt::AltTaskType, task_ws::WsChannel},
        traits::strategy::{CommandEmitter, EventHandler, Strategy},
    };

    use super::*;

    fn scheduler_key(task_id: u64) -> TaskKey {
        TaskKey::alt(&AltTaskType::TimeScheduler(Duration::from_secs(1)), task_id)
    }

    fn schedule_event(task_id: u64) -> TaskEvent {
        TaskEvent::Schedule(InfraMsg {
            task_id,
            data: Arc::new(AltScheduleEvent {
                timestamp: task_id,
                duration: Duration::from_secs(1),
            }),
        })
    }

    fn task_receiver(
        key: TaskKey,
        capacity: usize,
    ) -> (
        broadcast::Sender<TaskEvent>,
        crate::arch::strategy_base::handler::task_channel::TaskReceiver,
    ) {
        let (sender, receiver) = broadcast::channel(capacity);
        (sender, TaskReceiver { key, receiver })
    }

    #[derive(Clone)]
    struct ScheduleProbe {
        events: mpsc::UnboundedSender<u64>,
    }

    impl Strategy for ScheduleProbe {
        async fn initialize(&mut self) {}
    }

    impl CommandEmitter for ScheduleProbe {
        fn command_init(&mut self, _registry: Arc<CommandRegistry>) {}

        fn command_registry(&self) -> Arc<CommandRegistry> {
            Arc::new(CommandRegistry::default())
        }
    }

    impl EventHandler for ScheduleProbe {
        async fn on_schedule(&mut self, msg: InfraMsg<AltScheduleEvent>) {
            let _ = self.events.send(msg.task_id);
        }
    }

    async fn exercise_handler_path(receiver_count: usize) {
        let mut senders = Vec::with_capacity(receiver_count);
        let mut receivers = Vec::with_capacity(receiver_count);
        for task_id in 1..=receiver_count as u64 {
            let (sender, receiver) = task_receiver(scheduler_key(task_id), 4);
            senders.push(sender);
            receivers.push(receiver);
        }

        let (events, mut received) = mpsc::unbounded_channel();
        let handler = tokio::spawn(strategy_handler_loop(ScheduleProbe { events }, receivers));
        tokio::task::yield_now().await;

        let expected_task_id = receiver_count as u64;
        senders[receiver_count - 1]
            .send(schedule_event(expected_task_id))
            .unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), received.recv())
                .await
                .unwrap(),
            Some(expected_task_id)
        );

        drop(senders);
        tokio::time::timeout(Duration::from_secs(1), handler)
            .await
            .unwrap()
            .unwrap();
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    enum MarketEvent {
        Trade(u64),
        Lob(u64),
    }

    #[derive(Clone)]
    struct MarketProbe {
        events: mpsc::UnboundedSender<MarketEvent>,
    }

    impl Strategy for MarketProbe {
        async fn initialize(&mut self) {}
    }

    impl CommandEmitter for MarketProbe {
        fn command_init(&mut self, _registry: Arc<CommandRegistry>) {}

        fn command_registry(&self) -> Arc<CommandRegistry> {
            Arc::new(CommandRegistry::default())
        }
    }

    impl EventHandler for MarketProbe {
        async fn on_trade(&mut self, msg: InfraMsg<Vec<WsTrade>>) {
            let _ = self.events.send(MarketEvent::Trade(msg.task_id));
        }

        async fn on_lob(&mut self, msg: InfraMsg<Vec<WsLob>>) {
            let _ = self.events.send(MarketEvent::Lob(msg.task_id));
        }
    }

    #[tokio::test]
    async fn handler_without_receivers_returns() {
        let (events, _received) = mpsc::unbounded_channel();

        tokio::time::timeout(
            Duration::from_secs(1),
            strategy_handler_loop(ScheduleProbe { events }, Vec::new()),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn handler_wakes_dispatches_and_exits_across_lane_counts() {
        for receiver_count in [1, 5, 20] {
            exercise_handler_path(receiver_count).await;
        }
    }

    #[tokio::test]
    async fn handler_resumes_after_lag_across_lane_counts() {
        for receiver_count in [1, 5, 20] {
            let mut senders = Vec::with_capacity(receiver_count);
            let mut receivers = Vec::with_capacity(receiver_count);
            for task_id in 1..=receiver_count as u64 {
                let (sender, receiver) = task_receiver(scheduler_key(task_id), 1);
                senders.push(sender);
                receivers.push(receiver);
            }

            let expected_task_id = receiver_count as u64;
            senders[receiver_count - 1]
                .send(schedule_event(expected_task_id))
                .unwrap();
            senders[receiver_count - 1]
                .send(schedule_event(expected_task_id))
                .unwrap();

            let (events, mut received) = mpsc::unbounded_channel();
            let handler = tokio::spawn(strategy_handler_loop(ScheduleProbe { events }, receivers));

            assert_eq!(
                tokio::time::timeout(Duration::from_secs(1), received.recv())
                    .await
                    .unwrap(),
                Some(expected_task_id)
            );

            drop(senders);
            tokio::time::timeout(Duration::from_secs(1), handler)
                .await
                .unwrap()
                .unwrap();
        }
    }

    #[test]
    fn lag_rate_limit_aggregates_suppressed_messages() {
        let mut last_report_at = None;
        let mut skipped_since_report = 0;

        assert_eq!(
            record_lag(&mut last_report_at, &mut skipped_since_report, 2),
            Some(2)
        );
        assert_eq!(
            record_lag(&mut last_report_at, &mut skipped_since_report, 3),
            None
        );

        last_report_at = Some(Instant::now() - LAG_REPORT_INTERVAL);
        assert_eq!(
            record_lag(&mut last_report_at, &mut skipped_since_report, 4),
            Some(7)
        );
    }

    #[tokio::test]
    async fn handler_dispatches_task_event_with_original_task_id() {
        let key = scheduler_key(7);
        let channels = TaskChannels::new([key.clone()]).unwrap();
        let receivers = channels.subscribe([key.clone()]).unwrap();
        let (events, mut received) = mpsc::unbounded_channel();
        let handler = tokio::spawn(strategy_handler_loop(ScheduleProbe { events }, receivers));

        channels
            .sender(&key)
            .unwrap()
            .send(schedule_event(7))
            .unwrap();

        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), received.recv())
                .await
                .unwrap(),
            Some(7)
        );

        handler.abort();
    }

    #[tokio::test]
    async fn handler_dispatches_trade_and_lob_primary_events() {
        let trade_key = TaskKey::ws(&WsChannel::Trades(None), 1);
        let lob_key = TaskKey::ws(&WsChannel::Lob(None), 2);
        let channels = TaskChannels::new([trade_key.clone(), lob_key.clone()]).unwrap();
        let receivers = channels
            .subscribe([trade_key.clone(), lob_key.clone()])
            .unwrap();
        let (events, mut received) = mpsc::unbounded_channel();
        let handler = tokio::spawn(strategy_handler_loop(MarketProbe { events }, receivers));

        channels
            .sender(&trade_key)
            .unwrap()
            .send(TaskEvent::Trade(InfraMsg {
                task_id: 1,
                data: Arc::new(Vec::new()),
            }))
            .unwrap();
        channels
            .sender(&lob_key)
            .unwrap()
            .send(TaskEvent::Lob(InfraMsg {
                task_id: 2,
                data: Arc::new(Vec::new()),
            }))
            .unwrap();

        let mut actual = HashSet::new();
        for _ in 0..2 {
            actual.insert(
                tokio::time::timeout(Duration::from_secs(1), received.recv())
                    .await
                    .unwrap()
                    .unwrap(),
            );
        }
        assert_eq!(
            actual,
            HashSet::from([MarketEvent::Trade(1), MarketEvent::Lob(2)])
        );

        handler.abort();
    }

    #[tokio::test]
    async fn adaptive_mux_services_cold_lane_within_n_minus_one_hot_events() {
        let mut senders = Vec::new();
        let mut receivers = Vec::new();
        for task_id in 1..=5 {
            let (sender, receiver) = task_receiver(scheduler_key(task_id), 16);
            senders.push(sender);
            receivers.push(receiver);
        }

        for _ in 0..8 {
            senders[0].send(schedule_event(1)).unwrap();
        }
        senders[4].send(schedule_event(5)).unwrap();

        let mut mux = TaskEventMux::new(receivers);
        let mut hot_events = 0;
        loop {
            let Some(MuxItem::Event(TaskEvent::Schedule(msg))) = mux.next().await else {
                panic!("unexpected mux item");
            };
            if msg.task_id == 5 {
                break;
            }
            hot_events += 1;
        }

        assert!(hot_events < senders.len());
    }

    #[tokio::test]
    async fn lagged_item_retains_its_task_key() {
        let key = scheduler_key(9);
        let (sender, receiver) = task_receiver(key.clone(), 1);
        sender.send(schedule_event(9)).unwrap();
        sender.send(schedule_event(9)).unwrap();
        let mut mux = TaskEventMux::new(vec![receiver]);

        assert!(matches!(
            mux.next().await,
            Some(MuxItem::Lagged {
                key: lagged_key,
                skipped: 1,
            }) if lagged_key == key
        ));
    }

    #[tokio::test]
    async fn mux_terminates_after_all_senders_close() {
        let (sender, receiver) = task_receiver(scheduler_key(1), 1);
        let mut mux = TaskEventMux::new(vec![receiver]);
        drop(sender);

        assert!(mux.next().await.is_none());
    }

    #[tokio::test]
    async fn pending_mux_wakes_when_any_lane_receives() {
        let (first_sender, first_receiver) = task_receiver(scheduler_key(1), 1);
        let (second_sender, second_receiver) = task_receiver(scheduler_key(2), 1);
        let (third_sender, third_receiver) = task_receiver(scheduler_key(3), 1);
        let mut mux = TaskEventMux::new(vec![first_receiver, second_receiver, third_receiver]);
        let waiter = tokio::spawn(async move { mux.next().await });
        tokio::task::yield_now().await;

        third_sender.send(schedule_event(3)).unwrap();

        assert!(matches!(
            tokio::time::timeout(Duration::from_secs(1), waiter)
                .await
                .unwrap()
                .unwrap(),
            Some(MuxItem::Event(TaskEvent::Schedule(InfraMsg {
                task_id: 3,
                ..
            })))
        ));
        drop(first_sender);
        drop(second_sender);
        drop(third_sender);
    }

    #[tokio::test]
    async fn handler_continues_after_hot_lane_closes() {
        let (first_sender, first_receiver) = task_receiver(scheduler_key(1), 1);
        let (second_sender, second_receiver) = task_receiver(scheduler_key(2), 1);
        let (events, mut received) = mpsc::unbounded_channel();
        let handler = tokio::spawn(strategy_handler_loop(
            ScheduleProbe { events },
            vec![first_receiver, second_receiver],
        ));
        tokio::task::yield_now().await;

        drop(first_sender);
        tokio::task::yield_now().await;
        second_sender.send(schedule_event(2)).unwrap();

        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), received.recv())
                .await
                .unwrap(),
            Some(2)
        );

        drop(second_sender);
        tokio::time::timeout(Duration::from_secs(1), handler)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn pending_mux_wakes_when_all_senders_close() {
        let (first_sender, first_receiver) = task_receiver(scheduler_key(1), 1);
        let (second_sender, second_receiver) = task_receiver(scheduler_key(2), 1);
        let mut mux = TaskEventMux::new(vec![first_receiver, second_receiver]);
        let waiter = tokio::spawn(async move { mux.next().await });
        tokio::task::yield_now().await;

        drop(first_sender);
        drop(second_sender);

        assert!(
            tokio::time::timeout(Duration::from_secs(1), waiter)
                .await
                .unwrap()
                .unwrap()
                .is_none()
        );
    }
}
