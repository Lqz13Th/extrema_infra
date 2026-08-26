/// A raw JSON message from an exchange-specific websocket task.
///
/// `timestamp` is the local runtime receive timestamp in Unix microseconds.
/// `raw_json` contains the complete top-level websocket frame so that the
/// exchange-specific protocol can be decoded by a checker or adapter.
#[derive(Clone, Debug)]
pub struct WsOtherMessage {
    pub timestamp: u64,
    pub raw_json: String,
}
