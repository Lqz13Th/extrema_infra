use tokio::sync::oneshot::Sender;

/// Acknowledgement status returned by runtime tasks.
///
/// Strategies can pass an expected status to [`CommandHandle::send_command`]
/// to sequence relay commands such as connect -> login -> subscribe. An
/// acknowledgement reports progress inside the local relay; it does not imply
/// exchange-level authentication, subscription, or order acceptance.
///
/// [`CommandHandle::send_command`]: crate::arch::strategy_base::command::command_core::CommandHandle::send_command
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AckStatus {
    /// The websocket relay completed the connection handshake.
    WsConnect,
    /// The relay consumed a websocket message command and attempted the write.
    ///
    /// The relay currently returns this status even when the socket write fails.
    WsMessage,
    /// The relay consumed a shutdown command and attempted its payload write.
    ///
    /// The relay currently returns this status even when the socket write fails.
    WsShutdown,
    /// An alt task auto-acknowledged an unexpected ack-bearing command.
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
