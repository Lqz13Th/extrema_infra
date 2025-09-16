use thiserror::Error;
use reqwest::Error as ReqwestError;
use serde_json::Error as SerdeJsonError;
use tungstenite::error::Error as WsError;

#[derive(Error, Debug)]
pub enum InfraError {
    #[error("REST API error: {0}")]
    RestApi(#[from] ReqwestError),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] WsError),

    #[error("JSON parse error: {0}")]
    Json(#[from] SerdeJsonError),

    #[error("API returned error: {0}")]
    ApiError(String),

    #[error("Failed to parse received data: {0}")]
    ParseData(String),

    #[error("API transfer data error: {0}")]
    ApiTransferData(String),

    #[error("Empty response from API")]
    EmptyResponse,

    #[error("Invalid secret key length")]
    SecretKeyLength,

    #[error("API not initialized")]
    ApiNotInitialized,

    #[error("Chunk distribution error")]
    ChunkDistribution,

    #[error("Unknown WebSocket subscription")]
    UnknownWsSubscription,

    #[error("Unimplemented method")]
    Unimplemented,

    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Request timed out")]
    Timeout,

    #[error("WebSocket disconnected, need reconnect")]
    WsDisconnected,

    #[error("Environment variable missing: {0}")]
    EnvVarMissing(String),

    #[error("Other error: {0}")]
    Other(String),
}

pub type InfraResult<T> = Result<T, InfraError>;
