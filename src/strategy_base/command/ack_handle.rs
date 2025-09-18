use tokio::sync::oneshot::Sender;

pub use crate::errors::{InfraError, InfraResult};


#[derive(Debug)]
pub enum AckStatus {
    Login,
    Connect,
    Subscribe,
    Unsubscribe,
    Shutdown,
    AltTask,
}

#[derive(Debug)]
pub struct AckHandle {
    tx: Option<Sender<InfraResult<AckStatus>>>,
}

impl AckHandle {
    pub fn new(tx: Sender<InfraResult<AckStatus>>) -> Self {
        AckHandle { tx: Some(tx) }
    }

    pub fn respond(mut self, res: InfraResult<AckStatus>) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(res);
        }
    }

    pub fn none() -> Self {
        AckHandle { tx: None }
    }
}
