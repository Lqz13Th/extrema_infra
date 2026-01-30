use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::errors::{InfraError, InfraResult};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RestResHyperliquid<T> {
    Ok { status: String, response: T },
    Error { status: String, message: String },
}

impl<T> RestResHyperliquid<T> {
    pub fn into_data(self) -> InfraResult<T> {
        match self {
            Self::Ok { status, response } => {
                if status == "ok" {
                    Ok(response)
                } else {
                    warn!(
                        "Hyperliquid REST error (status={}): {:?}",
                        status, "unknown"
                    );
                    Err(InfraError::ApiCliError(format!(
                        "Hyperliquid REST error (status={})",
                        status
                    )))
                }
            },
            Self::Error { status, message } => {
                warn!("Hyperliquid REST error {}: {}", status, message);
                Err(InfraError::ApiCliError(format!(
                    "Hyperliquid REST error (status={}): {}",
                    status, message
                )))
            },
        }
    }
}
