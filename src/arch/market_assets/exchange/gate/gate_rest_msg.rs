use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::arch::traits::conversion::IntoInfraVec;
use crate::errors::{InfraError, InfraResult};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RestResGate<T> {
    Error { label: String, message: String },
    Data(Vec<T>),
    Object(T),
    DataField { data: Option<Vec<T>> },
    ObjectField { data: Option<T> },
}

impl<T> IntoInfraVec<T> for RestResGate<T> {
    fn into_vec(self) -> InfraResult<Vec<T>> {
        match self {
            Self::Data(v) => Ok(v),
            Self::Object(o) => Ok(vec![o]),
            Self::DataField { data } => Ok(data.unwrap_or_default()),
            Self::ObjectField { data } => Ok(data.into_iter().collect()),
            Self::Error { label, message } => {
                warn!("Gate REST error {}: {}", label, message);
                Err(InfraError::ApiCliError(format!(
                    "Gate REST error (label={}): {}",
                    label, message
                )))
            },
        }
    }
}
