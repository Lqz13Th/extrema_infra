use std::fmt::Debug;
use serde::Deserialize;
use tracing::{info, warn};

use crate::traits::conversion::IntoWsData;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum BinanceWsData<T> {
    SubscriptionResult(BinanceWsRes),
    ChannelData(T),
}

#[derive(Debug, Deserialize)]
pub struct BinanceWsRes {
    pub result: Option<String>,
    pub id: u32,
    pub error: Option<BinanceWsError>,
}

#[derive(Debug, Deserialize)]
pub struct BinanceWsError {
    pub code: i64,
    pub msg: String,
}
impl<T> IntoWsData for BinanceWsData<T>
where
    T: IntoWsData + for<'de> Deserialize<'de>,
    T::Output: Default,
{
    type Output = T::Output;
    fn into_ws(self) -> Self::Output {

        match self {
            BinanceWsData::ChannelData(channel_data) => {
                channel_data.into_ws()
            },
            BinanceWsData::SubscriptionResult(res) => {
                if let Some(result) = &res.result {
                    info!("Subscription method: {}. id = {}", result, res.id);
                } else {
                    info!("Subscription received. id = {}", res.id);
                }

                if let Some(err) = &res.error {
                    warn!(
                        "Subscription error. code = {}, msg = {}, id = {}",
                        err.code,
                        err.msg,
                        res.id
                    );
                }

                Default::default()
            }
        }

    }
}
