use serde::Deserialize;
use tracing::{info, warn};

use crate::arch::traits::conversion::IntoWsData;

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum BinanceWsData<T> {
    ChannelSingle(T),
    ChannelBatch(Vec<T>),
    Event(BinanceWsRes),
}

#[derive(Clone, Debug, Deserialize)]
pub struct BinanceWsRes {
    pub result: Option<String>,
    pub id: u32,
    pub error: Option<BinanceWsError>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BinanceWsError {
    pub code: i64,
    pub msg: String,
}
impl<T> IntoWsData for BinanceWsData<T>
where
    T: IntoWsData + for<'de> Deserialize<'de>,
{
    type Output = Vec<T::Output>;
    fn into_ws(self) -> Self::Output {

        match self {
            BinanceWsData::ChannelSingle(c) => vec![c.into_ws()],
            BinanceWsData::ChannelBatch(c) => c.into_iter().map(|d| d.into_ws()).collect(),
            BinanceWsData::Event(res) => {
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

                Vec::new()
            },
        }
    }
}
