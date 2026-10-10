use crate::errors::InfraResult;

/// Converts one decoded websocket frame into the payload published to strategies.
///
/// Built-in exchange schemas implement it, and so do frames of venues
/// implemented outside this crate, since it bounds the decoder passed to
/// [`WsFrameRunner::ws_loop`](crate::arch::traits::market_lob::WsFrameRunner::ws_loop).
pub trait IntoWsData {
    /// Payload type carried by the matching `TaskEvent` variant.
    type Output;
    /// Converts the decoded frame.
    fn into_ws(self) -> Self::Output;
}

/// Unwraps an exchange REST response envelope into infra data, mapping the
/// venue's error payload to [`InfraError`](crate::errors::InfraError).
pub trait IntoInfraData<T> {
    /// All items of a list payload; a single object becomes a one-item list.
    fn into_vec(self) -> InfraResult<Vec<T>>;

    /// Exactly one item, for single-object endpoints. An empty or multi-item
    /// payload is an error rather than a truncated result.
    fn into_one(self) -> InfraResult<T>;
}

/// The single element of a list payload; none or several is an error.
#[cfg(any(
    feature = "hyperliquid",
    feature = "binance",
    feature = "gate",
    feature = "okx"
))]
pub(crate) fn exactly_one<T>(items: Vec<T>) -> InfraResult<T> {
    let mut items = items.into_iter();
    match (items.next(), items.next()) {
        (Some(item), None) => Ok(item),
        (None, _) => Err(crate::errors::InfraError::ApiCliError(
            "expected exactly one item, got none".to_string(),
        )),
        (Some(_), Some(_)) => Err(crate::errors::InfraError::ApiCliError(
            "expected exactly one item, got several".to_string(),
        )),
    }
}
