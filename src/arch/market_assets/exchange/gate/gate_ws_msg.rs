use serde::Deserialize;
use serde_json::Value;
use tracing::{info, warn};

use crate::arch::traits::conversion::IntoWsData;

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum GateWsData<T> {
    Channel(GateWsChannel<T>),
    Event(GateWsEvent),
}

#[derive(Clone, Debug, Deserialize)]
pub struct GateWsChannel<T> {
    pub channel: String,
    pub event: String,
    pub result: Vec<T>,
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

impl<T> IntoWsData for GateWsData<T>
where
    T: IntoWsData + for<'de> Deserialize<'de>,
{
    type Output = Vec<T::Output>;

    fn into_ws(self) -> Self::Output {
        match self {
            GateWsData::Channel(c) => c.result.into_iter().map(|d| d.into_ws()).collect(),
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
