use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, oneshot};

use crate::arch::{
    strategy_base::{
        command::ack_handle::{AckHandle, AckStatus},
        handler::alt_events::{AltIntent, AltOrder, AltTensor},
    },
    task_execution::{task_alt::AltTaskType, task_general::TaskInfo, task_ws::WsChannel},
};
use crate::errors::{InfraError, InfraResult};

/// Lookup key used by [`CommandRegistry`].
///
/// Alt task commands are routed by `(AltTaskType, task_id)`. Websocket task
/// commands are routed by `(WsChannel, task_id)`. The market is part of the
/// task descriptor, but it is not part of the command key, so task ids must be
/// unique when several tasks share the same task type or websocket channel.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CommandKey {
    /// Command key for a non-websocket task.
    Alt {
        /// Alt task kind such as scheduler, model prediction, order execution,
        /// or instrument intent.
        alt_task_type: AltTaskType,
        /// Runtime task id assigned by `EnvBuilder`.
        task_id: u64,
    },
    /// Command key for a websocket relay task.
    Ws {
        /// Websocket channel such as trades, candles, account orders, or
        /// account positions.
        ws_channel: WsChannel,
        /// Runtime task id assigned by `EnvBuilder`.
        task_id: u64,
    },
}

/// Registry of command handles for all runtime-owned tasks.
///
/// The runtime creates this after spawning tasks and passes it into strategies
/// through `CommandEmitter::command_init`. Strategy modules store the registry
/// and use it to find the task they want to command.
#[derive(Clone, Debug, Default)]
pub struct CommandRegistry {
    handles: Arc<HashMap<CommandKey, Arc<CommandHandle>>>,
}

impl CommandRegistry {
    /// Builds a registry from task command handles.
    ///
    /// Panics if two handles produce the same [`CommandKey`]. In practice this
    /// means task ids must be unique for repeated alt task types or repeated
    /// websocket channels.
    pub fn new(handles: Vec<Arc<CommandHandle>>) -> Self {
        let mut map = HashMap::with_capacity(handles.len());

        for handle in handles {
            let key = match &handle.task_info {
                TaskInfo::AltTask(task) => CommandKey::Alt {
                    alt_task_type: task.alt_task_type.clone(),
                    task_id: handle.task_id,
                },
                TaskInfo::WsTask(task) => CommandKey::Ws {
                    ws_channel: task.ws_channel.clone(),
                    task_id: handle.task_id,
                },
            };

            if let Some(old) = map.insert(key.clone(), handle.clone()) {
                panic!(
                    "Duplicate CommandKey in registry: {:?}, old={:?}, new={:?}",
                    key, old, handle
                );
            }
        }

        Self {
            handles: Arc::new(map),
        }
    }

    /// Finds a non-websocket task handle.
    ///
    /// Use this for `TaskCommand::OrderExecute`, `TaskCommand::InstIntent`, or
    /// `TaskCommand::FeatInput` depending on the task type.
    pub fn find_alt_handle(
        &self,
        alt_task_type: &AltTaskType,
        task_id: u64,
    ) -> Option<Arc<CommandHandle>> {
        self.handles
            .get(&CommandKey::Alt {
                alt_task_type: alt_task_type.clone(),
                task_id,
            })
            .cloned()
    }

    /// Finds a websocket task handle.
    ///
    /// Use this after `on_ws_event` to send `TaskCommand::WsConnect`,
    /// `TaskCommand::WsMessage`, or `TaskCommand::WsShutdown` to the relay.
    pub fn find_ws_handle(
        &self,
        ws_channel: &WsChannel,
        task_id: u64,
    ) -> Option<Arc<CommandHandle>> {
        self.handles
            .get(&CommandKey::Ws {
                ws_channel: ws_channel.clone(),
                task_id,
            })
            .cloned()
    }
}

/// Command sender for one runtime task instance.
///
/// A strategy never writes to task channels directly. It looks up a
/// `CommandHandle` from [`CommandRegistry`] and calls
/// [`CommandHandle::send_command`] with a [`TaskCommand`].
#[derive(Clone, Debug)]
pub struct CommandHandle {
    /// Internal channel used to send commands into the task.
    pub cmd_tx: mpsc::Sender<TaskCommand>,
    /// Task declaration that owns this handle.
    pub task_info: TaskInfo,
    /// Runtime task id for this handle.
    pub task_id: u64,
}

impl CommandHandle {
    /// Sends one command to the task and optionally waits for an acknowledgement.
    ///
    /// Use `expected_ack = None` for fire-and-forget commands, such as many
    /// websocket subscription messages or task-local intent/order/model
    /// commands. Use `Some((AckStatus::..., rx))` when the strategy needs to
    /// confirm that a websocket connect/login/subscribe/shutdown step was
    /// accepted before sending the next message.
    ///
    /// Examples of common command flows:
    ///
    /// - websocket startup: `WsConnect` -> optional login `WsMessage` ->
    ///   subscription `WsMessage`;
    /// - websocket shutdown: `WsShutdown`;
    /// - execution module: `OrderExecute(Vec<AltOrder>)`;
    /// - portfolio/allocator module: `InstIntent(AltIntent)`;
    /// - model worker: `FeatInput(AltTensor)`, followed later by an
    ///   `EventHandler::on_preds` callback.
    pub async fn send_command(
        &self,
        cmd: TaskCommand,
        expected_ack: Option<(AckStatus, oneshot::Receiver<AckStatus>)>,
    ) -> InfraResult<()> {
        self.cmd_tx
            .send(cmd)
            .await
            .map_err(|e| InfraError::Msg(format!("Failed to send Command: {}", e)))?;

        if let Some((expected, rx)) = expected_ack {
            let ack = rx.await.map_err(|_| {
                InfraError::Msg(format!("Ack channel closed, expected ack: {:?}", expected,))
            })?;

            if ack == expected {
                Ok(())
            } else {
                Err(InfraError::Msg(format!(
                    "Unexpected ack: {:?}, expected: {:?}",
                    ack, expected,
                )))
            }
        } else {
            Ok(())
        }
    }
}

/// Command sent from a strategy module to a runtime task.
///
/// Commands are active requests. They are different from [`EventHandler`]
/// callbacks, which are passive inbound events emitted by tasks. The common
/// pattern is:
///
/// 1. a strategy receives an event, such as `on_ws_event` or `on_schedule`;
/// 2. it finds the relevant [`CommandHandle`];
/// 3. it sends a `TaskCommand`;
/// 4. the task performs work and may publish a later event.
///
/// Websocket commands carry exchange-specific strings that are built by the
/// concrete exchange client. For example, OKX private streams usually need a
/// `WsConnect`, then a login `WsMessage`, then a subscribe `WsMessage`.
/// Binance private streams often use listen-key URLs/messages, while public
/// trade/candle streams usually only need connect and subscribe.
///
/// Alt-task commands carry normalized infra data:
///
/// - `OrderExecute` forwards an order batch to an order-execution task, which
///   then emits `on_order_execution`.
/// - `InstIntent` forwards allocation/instrument/portfolio intent, which then
///   emits `on_inst_intent`.
/// - `FeatInput` forwards model features to a model task; predictions are later
///   emitted through `on_preds`.
///
/// [`EventHandler`]: crate::arch::traits::strategy::EventHandler
#[derive(Debug)]
pub enum TaskCommand {
    /// Opens a websocket connection.
    ///
    /// `msg` is usually the websocket URL returned by an exchange client's
    /// connect-message helper. The websocket relay waits for this command
    /// before it starts network IO.
    WsConnect {
        /// Websocket URL or exchange-specific connect target.
        msg: String,
        /// Acknowledgement handle, usually expected as `AckStatus::WsConnect`.
        ack: AckHandle,
    },

    /// Sends an arbitrary websocket text message through an open relay.
    ///
    /// Use this for login, authentication, subscription, unsubscription, ping,
    /// or other exchange-specific control messages. The message body is built
    /// by the concrete exchange client.
    WsMessage {
        /// Exchange-specific websocket message body.
        msg: String,
        /// Optional acknowledgement handle. Use `AckHandle::none()` when the
        /// caller does not need to wait.
        ack: AckHandle,
    },

    /// Requests websocket shutdown.
    ///
    /// `msg` can carry an exchange-specific close/unsubscribe payload when the
    /// exchange needs one before closing.
    WsShutdown {
        /// Exchange-specific shutdown or close message.
        msg: String,
        /// Optional acknowledgement handle.
        ack: AckHandle,
    },

    /// Sends a normalized batch of orders to an order-execution task.
    ///
    /// The receiving task publishes the same batch into the order-execution
    /// broadcast channel, where execution modules can handle it through
    /// `EventHandler::on_order_execution`.
    OrderExecute(Vec<AltOrder>),

    /// Sends an instrument, allocation, or portfolio intent to an intent task.
    ///
    /// The receiving task publishes the intent into the instrument-intent
    /// broadcast channel, where modules can handle it through
    /// `EventHandler::on_inst_intent`.
    InstIntent(AltIntent),

    /// Sends model features to a prediction task.
    ///
    /// The receiving model task runs inference through the configured backend
    /// (`ModelRunner::Zmq` or `ModelRunner::Onnx`) and later emits predictions
    /// through `EventHandler::on_preds`.
    FeatInput(AltTensor),
}

impl TaskCommand {
    /// Extracts an acknowledgement handle from commands that carry one.
    ///
    /// This is used internally when a task receives an unexpected command and
    /// still wants to unblock a caller waiting on an ack.
    pub fn get_ack(self) -> Option<AckHandle> {
        match self {
            TaskCommand::WsMessage { ack, .. } | TaskCommand::WsShutdown { ack, .. } => Some(ack),
            _ => None,
        }
    }
}
