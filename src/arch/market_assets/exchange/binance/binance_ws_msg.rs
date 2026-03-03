use serde::Deserialize;
use serde_json::Value;
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
    pub status: Option<u16>,
    pub result: Option<Value>,
    pub id: Value,
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
                let id = res.id.to_string();
                if let Some(result) = &res.result {
                    info!(
                        "Subscription received. status = {:?}, result = {}, id = {}",
                        res.status, result, id
                    );
                } else {
                    info!(
                        "Subscription received. status = {:?}, id = {}",
                        res.status, id
                    );
                }

                if let Some(err) = &res.error {
                    warn!(
                        "Subscription error. code = {}, msg = {}, id = {}",
                        err.code, err.msg, id
                    );
                }

                Vec::new()
            },
        }
    }
}
