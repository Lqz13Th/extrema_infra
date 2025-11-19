use tokio::sync::oneshot::Sender;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AckStatus {
    WsConnect,
    WsMessage,
    WsShutdown,
    AltTask,
    Unknown,
}

#[derive(Debug)]
pub struct AckHandle {
    tx: Option<Sender<AckStatus>>,
}

impl AckHandle {
    pub fn new(tx: Sender<AckStatus>) -> Self {
        AckHandle { tx: Some(tx) }
    }

    pub fn respond(mut self, res: AckStatus) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(res);
        }
    }

    pub fn none() -> Self {
        AckHandle { tx: None }
    }
}
