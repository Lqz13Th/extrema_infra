use std::sync::Arc;

use crate::errors::{InfraError, InfraResult};

use super::{
    task_alt::{AltTaskInfo, AltTaskType},
    task_ws::{WsChannel, WsTaskInfo},
};

/// Identity of one runtime task instance.
///
/// An alt task is identified by its complete [`AltTaskType`] and task id. A
/// websocket task is identified by its complete [`WsChannel`] and task id, so
/// embedded configuration such as scheduler duration or trade-stream parameter
/// is part of the key. Market, chunk, and task-base-id configuration are not.
///
/// Event callbacks receive the task id in [`InfraMsg`], not the full `TaskKey`.
/// For that reason, [`EnvBuilder`] rejects duplicate task ids for the same task
/// type, even when embedded configuration differs. For example, two `Trades`
/// channels or two schedulers cannot share an id, while Trade and LOB can.
///
/// [`EnvBuilder`]: crate::arch::infra_core::env_builder::EnvBuilder
/// [`InfraMsg`]: crate::arch::strategy_base::handler::task_channel::InfraMsg
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TaskKey {
    /// Identity of one non-websocket runtime task.
    Alt {
        /// Complete non-websocket task type and its embedded configuration.
        alt_task_type: AltTaskType,
        /// Runtime task id assigned by the declaration.
        task_id: u64,
    },
    /// Identity of one websocket runtime task.
    Ws {
        /// Complete websocket channel and its embedded configuration.
        ws_channel: WsChannel,
        /// Runtime task id assigned by the declaration.
        task_id: u64,
    },
}

impl TaskKey {
    /// Creates a websocket task identity from its complete channel and task id.
    pub fn ws(ws_channel: &WsChannel, task_id: u64) -> Self {
        Self::Ws {
            ws_channel: ws_channel.clone(),
            task_id,
        }
    }

    /// Creates an alt-task identity from its complete type and task id.
    pub fn alt(alt_task_type: &AltTaskType, task_id: u64) -> Self {
        Self::Alt {
            alt_task_type: alt_task_type.clone(),
            task_id,
        }
    }
}

/// Runtime task declaration accepted by [`EnvBuilder`].
///
/// [`EnvBuilder`]: crate::arch::infra_core::env_builder::EnvBuilder
#[derive(Clone, Debug)]
pub enum TaskInfo {
    /// Non-websocket task.
    AltTask(Arc<AltTaskInfo>),
    /// Websocket relay task.
    WsTask(Arc<WsTaskInfo>),
}

impl From<AltTaskInfo> for TaskInfo {
    fn from(task: AltTaskInfo) -> Self {
        Self::AltTask(Arc::new(task))
    }
}

impl From<WsTaskInfo> for TaskInfo {
    fn from(task: WsTaskInfo) -> Self {
        Self::WsTask(Arc::new(task))
    }
}

fn expand_task_ids(chunk: u64, task_base_id: Option<u64>) -> InfraResult<Vec<u64>> {
    if chunk == 0 {
        return Ok(Vec::new());
    }

    let first = task_base_id.unwrap_or(1);
    first.checked_add(chunk - 1).ok_or_else(|| {
        InfraError::Msg(format!(
            "task id range overflows u64: first={first}, chunk={chunk}"
        ))
    })?;

    Ok((0..chunk).map(|offset| first + offset).collect())
}

fn expand_task_keys(
    chunk: u64,
    task_base_id: Option<u64>,
    task_key: impl Fn(u64) -> TaskKey,
) -> InfraResult<Vec<TaskKey>> {
    Ok(expand_task_ids(chunk, task_base_id)?
        .into_iter()
        .map(task_key)
        .collect())
}

impl WsTaskInfo {
    /// Expands this declaration into all concrete task identities.
    ///
    /// IDs start at `task_base_id`, or at `1` when it is `None`, and continue
    /// consecutively for `chunk` entries. A zero `chunk` returns an empty vector.
    ///
    /// Returns an error when the configured task-id range exceeds `u64`.
    pub fn task_keys(&self) -> InfraResult<Vec<TaskKey>> {
        expand_task_keys(self.chunk, self.task_base_id, |task_id| {
            TaskKey::ws(&self.ws_channel, task_id)
        })
    }
}

impl AltTaskInfo {
    /// Expands this declaration into all concrete task identities.
    ///
    /// IDs start at `task_base_id`, or at `1` when it is `None`, and continue
    /// consecutively for `chunk` entries. A zero `chunk` returns an empty vector.
    ///
    /// Returns an error when the configured task-id range exceeds `u64`.
    pub fn task_keys(&self) -> InfraResult<Vec<TaskKey>> {
        expand_task_keys(self.chunk, self.task_base_id, |task_id| {
            TaskKey::alt(&self.alt_task_type, task_id)
        })
    }
}

impl TaskInfo {
    pub(crate) fn task_ids(&self) -> InfraResult<Vec<u64>> {
        let (chunk, task_base_id) = self.task_range();
        expand_task_ids(chunk, task_base_id)
    }

    /// Returns the identity of this task declaration for one runtime task id.
    pub fn task_key(&self, task_id: u64) -> TaskKey {
        match self {
            Self::WsTask(task) => TaskKey::ws(&task.ws_channel, task_id),
            Self::AltTask(task) => TaskKey::alt(&task.alt_task_type, task_id),
        }
    }

    /// Expands this declaration into all concrete task identities.
    ///
    /// Returns an error when the configured task-id range exceeds `u64`.
    pub fn task_keys(&self) -> InfraResult<Vec<TaskKey>> {
        match self {
            Self::WsTask(task) => task.task_keys(),
            Self::AltTask(task) => task.task_keys(),
        }
    }

    fn task_range(&self) -> (u64, Option<u64>) {
        match self {
            Self::WsTask(task) => (task.chunk, task.task_base_id),
            Self::AltTask(task) => (task.chunk, task.task_base_id),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum LogLevel {
    Info,
    Warn,
    Error,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::arch::{
        market_assets::market_core::Market,
        task_execution::{
            TaskKey,
            task_alt::AltTaskType,
            task_ws::{TradesParam, WsChannel, WsTaskInfo},
        },
    };

    use super::*;

    #[test]
    fn task_keys_expand_chunk_from_base_id() {
        let task = TaskInfo::WsTask(Arc::new(WsTaskInfo {
            market: Market::BinanceSpot,
            ws_channel: WsChannel::Trades(None),
            filter_channels: false,
            chunk: 3,
            task_base_id: Some(10),
        }));

        assert_eq!(
            task.task_keys().unwrap(),
            vec![
                TaskKey::ws(&WsChannel::Trades(None), 10),
                TaskKey::ws(&WsChannel::Trades(None), 11),
                TaskKey::ws(&WsChannel::Trades(None), 12),
            ]
        );
    }

    #[test]
    fn raw_ws_task_keys_expand_chunk_from_base_id() {
        let task = WsTaskInfo {
            market: Market::BinanceSpot,
            ws_channel: WsChannel::Trades(None),
            filter_channels: false,
            chunk: 3,
            task_base_id: Some(10),
        };

        assert_eq!(
            task.task_keys().unwrap(),
            vec![
                TaskKey::ws(&WsChannel::Trades(None), 10),
                TaskKey::ws(&WsChannel::Trades(None), 11),
                TaskKey::ws(&WsChannel::Trades(None), 12),
            ]
        );
    }

    #[test]
    fn raw_alt_task_keys_expand_chunk_from_base_id() {
        let task = AltTaskInfo {
            alt_task_type: AltTaskType::OrderExecution,
            chunk: 2,
            task_base_id: Some(20),
        };

        assert_eq!(
            task.task_keys().unwrap(),
            vec![
                TaskKey::alt(&AltTaskType::OrderExecution, 20),
                TaskKey::alt(&AltTaskType::OrderExecution, 21),
            ]
        );
    }

    #[test]
    fn maximum_task_id_is_valid_for_one_instance() {
        let task = TaskInfo::WsTask(Arc::new(WsTaskInfo {
            market: Market::BinanceSpot,
            ws_channel: WsChannel::Trades(None),
            filter_channels: false,
            chunk: 1,
            task_base_id: Some(u64::MAX),
        }));

        assert_eq!(task.task_ids().unwrap(), vec![u64::MAX]);
    }

    #[test]
    fn overflowing_task_id_range_is_rejected() {
        let task = TaskInfo::WsTask(Arc::new(WsTaskInfo {
            market: Market::BinanceSpot,
            ws_channel: WsChannel::Trades(None),
            filter_channels: false,
            chunk: 2,
            task_base_id: Some(u64::MAX),
        }));

        assert!(task.task_ids().is_err());
        assert!(task.task_keys().is_err());
    }

    #[test]
    fn websocket_channel_configuration_is_part_of_identity() {
        let aggregated = TaskKey::ws(&WsChannel::Trades(Some(TradesParam::AggTrades)), 7);
        let raw = TaskKey::ws(&WsChannel::Trades(Some(TradesParam::AllTrades)), 7);

        assert_ne!(aggregated, raw);
    }

    #[test]
    fn alt_task_configuration_is_part_of_identity() {
        let fast = TaskKey::alt(&AltTaskType::TimeScheduler(Duration::from_secs(1)), 7);
        let slow = TaskKey::alt(&AltTaskType::TimeScheduler(Duration::from_secs(5)), 7);

        assert_ne!(fast, slow);
    }
}
