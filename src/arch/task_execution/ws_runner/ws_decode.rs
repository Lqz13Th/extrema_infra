use crate::arch::{
    market_assets::api_general::get_micros_timestamp,
    strategy_base::handler::events::ws_events::WsOtherMessage, traits::conversion::IntoWsData,
};
use serde::de::DeserializeOwned;

/// Decodes an exchange-specific JSON websocket frame without interpreting its
/// exchange-specific fields.
///
pub(crate) fn decode_raw_ws(frame: &[u8]) -> serde_json::Result<WsOtherMessage> {
    let timestamp = get_micros_timestamp();
    serde_json::from_slice::<serde_json::Value>(frame)?;

    Ok(WsOtherMessage {
        timestamp,
        raw_json: String::from_utf8_lossy(frame).into_owned(),
    })
}

impl IntoWsData for WsOtherMessage {
    type Output = Vec<WsOtherMessage>;

    fn into_ws(self) -> Self::Output {
        vec![self]
    }
}

/// Tries the runner's expected data shape before falling back to the full frame.
pub(crate) fn decode_preferred<Frame, Preferred, Wrap>(
    frame: &[u8],
    wrap: Wrap,
) -> serde_json::Result<Frame>
where
    Frame: DeserializeOwned,
    Preferred: DeserializeOwned,
    Wrap: FnOnce(Preferred) -> Frame,
{
    match serde_json::from_slice::<Preferred>(frame) {
        Ok(message) => Ok(wrap(message)),
        Err(error) => {
            tracing::trace!(
                preferred = std::any::type_name::<Preferred>(),
                error = %error,
                "WS preferred decoder fallback"
            );
            serde_json::from_slice(frame)
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::arch::traits::conversion::IntoWsData;
    use serde::Deserialize;

    use super::{decode_preferred, decode_raw_ws};

    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum TestFrame {
        Data(TestData),
        Event(TestEvent),
    }

    #[derive(Debug, Deserialize)]
    struct TestData {
        value: u64,
    }

    #[derive(Debug, Deserialize)]
    struct TestEvent {
        event: String,
    }

    #[test]
    fn decodes_preferred_shape_and_falls_back_to_full_frame() {
        let data = decode_preferred::<TestFrame, TestData, _>(br#"{"value":42}"#, TestFrame::Data)
            .unwrap();
        let event = decode_preferred::<TestFrame, TestData, _>(
            br#"{"event":"subscribe"}"#,
            TestFrame::Data,
        )
        .unwrap();

        assert!(matches!(data, TestFrame::Data(TestData { value: 42 })));
        assert!(matches!(
            event,
            TestFrame::Event(TestEvent { event }) if event == "subscribe"
        ));
    }

    #[test]
    fn returns_the_full_frame_decoder_error() {
        let frame = br#"{"value":"#;
        let expected = serde_json::from_slice::<TestFrame>(frame)
            .unwrap_err()
            .to_string();
        let actual = decode_preferred::<TestFrame, TestData, _>(frame, TestFrame::Data)
            .unwrap_err()
            .to_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn raw_decoder_preserves_complete_json_frame_and_receive_time() {
        let frame = br#"{ "channel": "post", "data": { "id": 256 } }"#;
        let data = decode_raw_ws(frame).unwrap();
        let message = data.into_ws().pop().expect("raw websocket message");

        assert_eq!(message.raw_json, std::str::from_utf8(frame).unwrap());
        assert!(message.raw_json.contains("\"channel\": \"post\""));
        assert!(message.timestamp > 0);
    }

    #[test]
    fn raw_decoder_rejects_invalid_json() {
        assert!(decode_raw_ws(br#"not-json"#).is_err());
    }
}
