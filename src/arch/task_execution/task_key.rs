use super::{task_alt::AltTaskType, task_ws::WsChannel};

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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::arch::task_execution::task_ws::TradesParam;

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
