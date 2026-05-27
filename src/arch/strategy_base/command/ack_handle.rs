use tokio::sync::oneshot::Sender;

/// Acknowledgement status returned by runtime tasks.
///
/// Strategies can pass an expected status to
/// `CommandHandle::send_command`. This is useful for ordered websocket control
/// flows where the next message should only be sent after the previous command
/// was accepted, such as connect -> login -> subscribe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AckStatus {
    /// Websocket connection command was accepted.
    WsConnect,
    /// Websocket message command was accepted.
    WsMessage,
    /// Websocket shutdown command was accepted.
    WsShutdown,
    /// Alt task command was accepted or auto-acknowledged.
    AltTask,
    /// Unknown or placeholder acknowledgement.
    Unknown,
}

/// One-shot acknowledgement sender attached to a [`TaskCommand`].
///
/// Use [`AckHandle::new`] when a strategy wants to wait for a task to
/// acknowledge a command. Use [`AckHandle::none`] for fire-and-forget commands.
///
/// [`TaskCommand`]: crate::arch::strategy_base::command::command_core::TaskCommand
#[derive(Debug)]
pub struct AckHandle {
    tx: Option<Sender<AckStatus>>,
}

impl AckHandle {
    /// Creates an acknowledgement handle backed by a one-shot sender.
    pub fn new(tx: Sender<AckStatus>) -> Self {
        AckHandle { tx: Some(tx) }
    }

    /// Sends an acknowledgement status if this handle has a sender.
    pub fn respond(mut self, res: AckStatus) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(res);
        }
    }

    /// Creates a no-op acknowledgement handle.
    pub fn none() -> Self {
        AckHandle { tx: None }
    }
}
