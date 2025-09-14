use tokio::sync::oneshot;


use crate::errors::*;

#[derive(Debug)]
pub struct AckHandle {
    tx: Option<oneshot::Sender<InfraResult<()>>>,
}

impl AckHandle {
    pub fn new(tx: oneshot::Sender<InfraResult<()>>) -> Self {
        AckHandle { tx: Some(tx) }
    }

    pub fn respond(mut self, res: InfraResult<()>) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(res);
        }
    }

    pub fn none() -> Self {
        AckHandle { tx: None }
    }
}
