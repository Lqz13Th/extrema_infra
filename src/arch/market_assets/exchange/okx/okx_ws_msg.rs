use serde::Deserialize;
use tracing::{info, warn};

use crate::arch::traits::conversion::IntoWsData;

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum OkxWsData<T> {
    ChannelBatch(OkxWsChannel<T>),
    Event(OkxWsEvent),
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct OkxWsEvent {
    pub event: Option<String>,
    pub code: Option<String>,
    pub msg: Option<String>,
    pub arg: Option<WsArg>,
    pub connCount: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct WsArg {
    pub channel: Option<String>,
    pub instId: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct OkxWsChannel<T> {
    pub arg: WsArg,
    pub data: Vec<T>,
}

impl<T> IntoWsData for OkxWsData<T>
where
    T: IntoWsData + for<'de> Deserialize<'de>,
{
    type Output = Vec<T::Output>;

    fn into_ws(self) -> Self::Output {
        match self {
            OkxWsData::ChannelBatch(c) => c.data.into_iter().map(|d| d.into_ws()).collect(),
            OkxWsData::Event(res) => {
                if let Some(event) = res.event {
                    match event.as_str() {
                        "subscribe" => {
                            info!("Subscribed channel: {:?}", res.arg);
                        },
                        "error" => {
                            warn!(
                                "Subscription error: code = {:?}, msg = {:?}",
                                res.code, res.msg
                            );
                        },
                        _ => {
                            info!(
                                "Other: {}, code: {:?}, msg: {:?}, arg: {:?}, count: {:?}",
                                event, res.code, res.msg, res.arg, res.connCount
                            );
                        }
                    };
                }

                Vec::new()
            },
        }
    }
}