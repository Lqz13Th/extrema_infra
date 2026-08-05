use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use tracing::{info, warn};

use crate::arch::{
    task_execution::ws_runner::ws_decode::decode_preferred, traits::conversion::IntoWsData,
};

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum GateWsData<T> {
    Channel(GateWsChannel<T>),
    Single(GateWsSingleChannel<T>),
    Event(GateWsEvent),
}

#[derive(Clone, Debug, Deserialize)]
pub struct GateWsChannel<T> {
    pub channel: String,
    pub event: String,
    pub result: Vec<T>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GateWsSingleChannel<T> {
    pub channel: String,
    pub event: String,
    pub result: T,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GateWsEvent {
    pub channel: Option<String>,
    pub event: Option<String>,
    pub error: Option<GateWsError>,
    pub result: Option<Value>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GateWsError {
    pub code: i64,
    pub message: String,
}

impl<T: DeserializeOwned> GateWsData<T> {
    pub(crate) fn decode_single(frame: &[u8]) -> serde_json::Result<Self> {
        decode_preferred(frame, Self::Single)
    }

    pub(crate) fn decode_batch(frame: &[u8]) -> serde_json::Result<Self> {
        decode_preferred(frame, Self::Channel)
    }
}

impl<T> IntoWsData for GateWsData<T>
where
    T: IntoWsData + for<'de> Deserialize<'de>,
{
    type Output = Vec<T::Output>;

    fn into_ws(self) -> Self::Output {
        match self {
            GateWsData::Channel(c) => c.result.into_iter().map(|d| d.into_ws()).collect(),
            GateWsData::Single(c) => vec![c.result.into_ws()],
            GateWsData::Event(res) => {
                if let Some(err) = res.error {
                    warn!(
                        "Gate WS error: code = {}, message = {}, channel = {:?}, event = {:?}",
                        err.code, err.message, res.channel, res.event
                    );
                } else if let Some(event) = res.event.as_deref() {
                    match event {
                        "subscribe" => {
                            info!("Gate WS subscribed: channel = {:?}", res.channel);
                        },
                        "unsubscribe" => {
                            info!("Gate WS unsubscribed: channel = {:?}", res.channel);
                        },
                        _ => {
                            info!(
                                "Gate WS event: {:?}, channel = {:?}, result = {:?}",
                                res.event, res.channel, res.result
                            );
                        },
                    }
                }

                Vec::new()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::GateWsData;

    #[derive(Debug, Deserialize)]
    struct TestPayload {
        value: u64,
    }

    #[test]
    fn preferred_decoders_preserve_control_and_error_frames() {
        let control = GateWsData::<TestPayload>::decode_single(
            br#"{"channel":"futures.book_ticker","event":"subscribe","result":{"status":"success"}}"#,
        )
        .unwrap();
        let error = GateWsData::<TestPayload>::decode_batch(
            br#"{"channel":"futures.trades","event":"subscribe","error":{"code":2,"message":"bad request"}}"#,
        )
        .unwrap();

        assert!(matches!(control, GateWsData::Event(_)));
        assert!(matches!(
            error,
            GateWsData::Event(super::GateWsEvent { error: Some(_), .. })
        ));
    }
}
