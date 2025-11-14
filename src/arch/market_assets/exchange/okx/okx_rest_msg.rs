use serde::Deserialize;
use tracing::warn;

use crate::errors::{InfraError, InfraResult};
use crate::arch::traits::conversion::IntoInfraVec;



#[derive(Clone, Debug, Deserialize)]
pub struct RestResOkx<T> {
    pub code: String,
    pub data: Option<Vec<T>>,
    pub msg: Option<String>,
}


impl<T> IntoInfraVec<T> for RestResOkx<T> {
    fn into_vec(self) -> InfraResult<Vec<T>> {
        if self.code != "0" {
            warn!("OKX REST error {}: {:?}", self.code, self.msg);
            return Err(InfraError::ApiCliError(format!(
                "OKX REST error (code={}): {:?}",
                self.code, self.msg
            )));
        }

        Ok(self.data.unwrap_or_default())
    }
}