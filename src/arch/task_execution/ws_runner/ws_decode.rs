use serde::de::DeserializeOwned;

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
    use serde::Deserialize;

    use super::decode_preferred;

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
}
