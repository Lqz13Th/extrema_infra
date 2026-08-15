use serde::Deserialize;
use tracing::warn;

use crate::arch::traits::conversion::IntoInfraVec;
use crate::errors::{InfraError, InfraResult};

#[derive(Clone, Debug, Deserialize)]
pub struct RestResOkx<T> {
    pub code: String,
    pub data: Option<Vec<T>>,
    pub msg: Option<String>,
}

impl<T: std::fmt::Debug> IntoInfraVec<T> for RestResOkx<T> {
    fn into_vec(self) -> InfraResult<Vec<T>> {
        if self.code != "0" {
            warn!(
                "OKX REST error {}: {:?}, data: {:?}",
                self.code, self.msg, self.data
            );
            return Err(InfraError::ApiCliError(format!(
                "OKX REST error (code={}): {:?}",
                self.code, self.msg
            )));
        }

        Ok(self.data.unwrap_or_default())
    }
}

impl<T: std::fmt::Debug> RestResOkx<T> {
    pub fn into_batch_vec(self) -> InfraResult<Vec<T>> {
        if !matches!(self.code.as_str(), "0" | "1" | "2") {
            warn!(
                "OKX REST batch error {}: {:?}, data: {:?}",
                self.code, self.msg, self.data
            );
            return Err(InfraError::ApiCliError(format!(
                "OKX REST batch error (code={}): {:?}",
                self.code, self.msg
            )));
        }

        Ok(self.data.unwrap_or_default())
    }
}
