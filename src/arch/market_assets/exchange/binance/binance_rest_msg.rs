use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::errors::{InfraError, InfraResult};
use crate::arch::traits::conversion::IntoInfraVec;


#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RestResBinance<T> {
    Error { code: i64, msg: String },
    Data(Vec<T>),
    Object(T),
}

impl<T> IntoInfraVec<T> for RestResBinance<T> {
    fn into_vec(self) -> InfraResult<Vec<T>> {
        match self {
            Self::Data(v) => Ok(v),
            Self::Object(o) => Ok(vec![o]),
            Self::Error { code, msg } => {
                warn!("Binance REST error {}: {}", code, msg);
                Err(InfraError::ApiError(format!(
                    "Binance REST error (code={}): {}",
                    code, msg
                )))
            }
        }
    }
}
