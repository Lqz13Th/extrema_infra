use std::{
    collections::{HashMap, HashSet},
    mem::{Discriminant, discriminant},
};

use tokio::sync::broadcast;

use crate::arch::{
    strategy_base::handler::events::{
        alt_events::{AltIntent, AltOrder, AltScheduleEvent, AltTensor},
        lob_events::{WsAccBalPos, WsAccOrder, WsAccPosition, WsCandle, WsLob, WsLobMbo, WsTrade},
        ws_events::WsOtherMessage,
    },
    task_execution::{
        TaskKey,
        task_alt::{AltTaskInfo, AltTaskType},
        task_ws::{WsChannel, WsTaskInfo},
    },
};
use crate::errors::{InfraError, InfraResult};

pub use super::events::InfraMsg;

const WS_EVENT_CHANNEL_CAPACITY: usize = 2_048;
const ORDER_EXECUTION_CHANNEL_CAPACITY: usize = 8_192;
const INST_INTENT_CHANNEL_CAPACITY: usize = 2_048;
#[cfg(any(feature = "model_onnx", feature = "model_zmq"))]
const MODEL_PREDS_CHANNEL_CAPACITY: usize = 8_192;
const SCHEDULE_CHANNEL_CAPACITY: usize = 1_024;
const TRADE_CHANNEL_CAPACITY: usize = 8_192;
const LOB_CHANNEL_CAPACITY: usize = 16_384;
const LOB_MBO_CHANNEL_CAPACITY: usize = 65_536;
const CANDLE_CHANNEL_CAPACITY: usize = 2_048;
const ACC_ORDER_CHANNEL_CAPACITY: usize = 8_192;
const ACC_BAL_POS_CHANNEL_CAPACITY: usize = 8_192;
const ACC_POS_CHANNEL_CAPACITY: usize = 8_192;

// Keep the event union complete across feature subsets and for callback types
// whose exchange producer is not implemented yet.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) enum TaskEvent {
    Alt(InfraMsg<AltTaskInfo>),
    Ws(InfraMsg<WsTaskInfo>),
    OrderExecute(InfraMsg<Vec<AltOrder>>),
    InstIntent(InfraMsg<AltIntent>),
    ModelPreds(InfraMsg<AltTensor>),
    Schedule(InfraMsg<AltScheduleEvent>),
    Trade(InfraMsg<Vec<WsTrade>>),
    Lob(InfraMsg<Vec<WsLob>>),
    LobMbo(InfraMsg<Vec<WsLobMbo>>),
    Candle(InfraMsg<Vec<WsCandle>>),
    AccOrder(InfraMsg<Vec<WsAccOrder>>),
    AccBalPos(InfraMsg<Vec<WsAccBalPos>>),
    AccPos(InfraMsg<Vec<WsAccPosition>>),
    WsOther(InfraMsg<Vec<WsOtherMessage>>),
}

pub(crate) struct TaskReceiver {
    pub(crate) key: TaskKey,
    pub(crate) receiver: broadcast::Receiver<TaskEvent>,
}

#[doc(hidden)]
pub struct TaskChannels {
    channels: HashMap<TaskKey, broadcast::Sender<TaskEvent>>,
}

impl TaskChannels {
    pub(crate) fn new(keys: impl IntoIterator<Item = TaskKey>) -> InfraResult<Self> {
        let mut channels = HashMap::new();
        let mut alt_task_ids: HashSet<(Discriminant<AltTaskType>, u64)> = HashSet::new();
        let mut ws_task_ids: HashSet<(Discriminant<WsChannel>, u64)> = HashSet::new();

        for key in keys {
            let is_unique = match &key {
                TaskKey::Alt {
                    alt_task_type,
                    task_id,
                } => alt_task_ids.insert((discriminant(alt_task_type), *task_id)),
                TaskKey::Ws {
                    ws_channel,
                    task_id,
                } => ws_task_ids.insert((discriminant(ws_channel), *task_id)),
            };

            if !is_unique {
                return Err(InfraError::Msg(format!(
                    "duplicate task id for the same task type: {key:?}"
                )));
            }

            let (sender, _) = broadcast::channel(capacity_for(&key));
            channels.insert(key, sender);
        }

        Ok(Self { channels })
    }

    pub(crate) fn subscribe(
        &self,
        keys: impl IntoIterator<Item = TaskKey>,
    ) -> InfraResult<Vec<TaskReceiver>> {
        let mut unique_keys = HashSet::new();
        let mut receivers = Vec::new();
        for key in keys {
            if !unique_keys.insert(key.clone()) {
                continue;
            }

            let sender = self.channels.get(&key).ok_or_else(|| {
                InfraError::Msg(format!("cannot subscribe to unknown task key: {key:?}"))
            })?;
            receivers.push(TaskReceiver {
                key,
                receiver: sender.subscribe(),
            });
        }

        Ok(receivers)
    }

    pub(crate) fn subscribe_all(&self) -> Vec<TaskReceiver> {
        self.channels
            .iter()
            .map(|(key, sender)| TaskReceiver {
                key: key.clone(),
                receiver: sender.subscribe(),
            })
            .collect()
    }

    pub(crate) fn sender(&self, key: &TaskKey) -> Option<broadcast::Sender<TaskEvent>> {
        self.channels.get(key).cloned()
    }

    pub(crate) fn contains(&self, key: &TaskKey) -> bool {
        self.channels.contains_key(key)
    }
}

fn capacity_for(key: &TaskKey) -> usize {
    match key {
        TaskKey::Alt { alt_task_type, .. } => match alt_task_type {
            AltTaskType::OrderExecution => ORDER_EXECUTION_CHANNEL_CAPACITY,
            AltTaskType::InstIntent => INST_INTENT_CHANNEL_CAPACITY,
            #[cfg(any(feature = "model_onnx", feature = "model_zmq"))]
            AltTaskType::ModelPreds(_) => MODEL_PREDS_CHANNEL_CAPACITY,
            AltTaskType::TimeScheduler(_) => SCHEDULE_CHANNEL_CAPACITY,
        },
        TaskKey::Ws { ws_channel, .. } => match ws_channel {
            WsChannel::AccountOrders => ACC_ORDER_CHANNEL_CAPACITY,
            WsChannel::AccountBalAndPos => ACC_BAL_POS_CHANNEL_CAPACITY,
            WsChannel::AccountPositions => ACC_POS_CHANNEL_CAPACITY,
            WsChannel::Candles(_) => CANDLE_CHANNEL_CAPACITY,
            WsChannel::Trades(_) => TRADE_CHANNEL_CAPACITY,
            WsChannel::Lob(_) => LOB_CHANNEL_CAPACITY,
            WsChannel::LobMbo => LOB_MBO_CHANNEL_CAPACITY,
            WsChannel::Other(_) => WS_EVENT_CHANNEL_CAPACITY,
        },
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

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

    #[test]
    fn task_rings_are_independent() {
        let first = scheduler_key(1);
        let second = scheduler_key(2);
        let channels = TaskChannels::new([first.clone(), second.clone()]).unwrap();
        let mut receivers = channels.subscribe([first.clone(), second.clone()]).unwrap();

        channels
            .sender(&first)
            .unwrap()
            .send(schedule_event(1))
            .unwrap();

        assert!(matches!(
            receivers[0].receiver.try_recv(),
            Ok(TaskEvent::Schedule(InfraMsg { task_id: 1, .. }))
        ));
        assert!(matches!(
            receivers[1].receiver.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));
    }

    #[test]
    fn one_task_broadcasts_to_multiple_receivers() {
        let key = scheduler_key(1);
        let channels = TaskChannels::new([key.clone()]).unwrap();
        let mut first = channels.subscribe([key.clone()]).unwrap().pop().unwrap();
        let mut second = channels.subscribe([key.clone()]).unwrap().pop().unwrap();

        channels
            .sender(&key)
            .unwrap()
            .send(schedule_event(1))
            .unwrap();

        assert!(first.receiver.try_recv().is_ok());
        assert!(second.receiver.try_recv().is_ok());
    }

    #[test]
    fn duplicate_task_key_fails() {
        let key = scheduler_key(1);

        assert!(TaskChannels::new([key.clone(), key]).is_err());
    }

    #[test]
    fn duplicate_subscription_keys_create_one_receiver() {
        let key = scheduler_key(1);
        let channels = TaskChannels::new([key.clone()]).unwrap();

        assert_eq!(channels.subscribe([key.clone(), key]).unwrap().len(), 1);
    }
}
