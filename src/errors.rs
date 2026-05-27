//! Error and result types used across the crate.
//!
//! Public APIs return [`InfraResult<T>`], which is a crate-local alias around
//! [`Result`] with [`InfraError`]. The error enum keeps exchange, parsing,
//! environment-variable, and unimplemented-endpoint failures in one type so
//! strategy code can propagate errors without depending on exchange-specific
//! error enums.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum InfraError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("REST API error: {0}")]
    RestApi(#[from] reqwest::Error),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] Box<tungstenite::Error>),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("JSON parse error (simd): {0}")]
    SimdJson(#[from] simd_json::Error),

    #[error("Polars error: {0}")]
    Polars(#[from] polars::error::PolarsError),

    #[error("API cli error: {0}")]
    ApiCliError(String),

    #[error("API cli not initialized")]
    ApiCliNotInitialized,

    #[error("Invalid secret key length")]
    SecretKeyLength,

    #[error("Environment variable missing: {0}")]
    EnvVarMissing(String),

    #[error("Unimplemented method")]
    Unimplemented,

    #[error("{0}")]
    Msg(String),
}

pub type InfraResult<T> = Result<T, InfraError>;
