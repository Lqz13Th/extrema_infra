use serde::Deserialize;
use tracing::{info, warn};

use crate::arch::traits::conversion::IntoWsData;

pub(crate) trait IntoOkxWsData {
    type Output;

    fn into_ws_with_okx_context(self, arg: &WsArg, action: Option<&str>) -> Self::Output;
}

impl<T> IntoOkxWsData for T
where
    T: IntoWsData,
{
    type Output = T::Output;

    fn into_ws_with_okx_context(self, _arg: &WsArg, _action: Option<&str>) -> Self::Output {
        self.into_ws()
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum OkxWsData<T> {
    ChannelBatch(OkxWsChannel<T>),
    Event(OkxWsEvent),
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct OkxWsEvent {
    pub event: Option<String>,
    pub code: Option<String>,
    pub msg: Option<String>,
    pub arg: Option<WsArg>,
    pub connCount: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsArg {
    pub channel: Option<String>,
    pub instId: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct OkxWsChannel<T> {
    pub arg: WsArg,
    pub action: Option<String>,
    pub data: Vec<T>,
}

impl<T> IntoWsData for OkxWsData<T>
where
    T: IntoOkxWsData + for<'de> Deserialize<'de>,
{
    type Output = Vec<T::Output>;

    fn into_ws(self) -> Self::Output {
        match self {
            OkxWsData::ChannelBatch(c) => {
                let action = c.action.as_deref();

                c.data
                    .into_iter()
                    .map(|d| d.into_ws_with_okx_context(&c.arg, action))
                    .collect()
            },
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
                        },
                    };
                }

                Vec::new()
            },
        }
    }
}
