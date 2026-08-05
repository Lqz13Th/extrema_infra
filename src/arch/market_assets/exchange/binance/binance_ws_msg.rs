use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use tracing::{info, warn};

use crate::arch::{
    task_execution::ws_runner::ws_decode::decode_preferred, traits::conversion::IntoWsData,
};

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum BinanceWsData<T> {
    ChannelSingle(T),
    ChannelBatch(Vec<T>),
    AccountPositions(BinanceWsAccountPositions<T>),
    Event(BinanceWsRes),
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct BinanceWsAccountPositions<T> {
    pub a: BinanceWsAccountPositionsData<T>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct BinanceWsAccountPositionsData<T> {
    pub P: Vec<T>,
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

impl<T: DeserializeOwned> BinanceWsData<T> {
    pub(crate) fn decode_single(frame: &[u8]) -> serde_json::Result<Self> {
        decode_preferred(frame, Self::ChannelSingle)
    }

    pub(crate) fn decode_account_positions(frame: &[u8]) -> serde_json::Result<Self> {
        decode_preferred(frame, Self::AccountPositions)
    }
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
            BinanceWsData::AccountPositions(c) => {
                c.a.P
                    .into_iter()
                    .map(|position| position.into_ws())
                    .collect()
            },
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

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::BinanceWsData;

    #[derive(Debug, Deserialize)]
    struct TestPayload {
        value: u64,
    }

    #[test]
    fn single_decoder_preserves_control_and_error_frames() {
        let cases: &[(&[u8], bool)] = &[
            (br#"{"status":200,"result":null,"id":42}"#, false),
            (
                br#"{"status":400,"id":42,"error":{"code":-1,"msg":"bad request"}}"#,
                true,
            ),
            (br#"{"code":-1,"msg":"bad request","id":42}"#, false),
        ];

        for (frame, has_nested_error) in cases {
            let BinanceWsData::Event(event) =
                BinanceWsData::<TestPayload>::decode_single(frame).unwrap()
            else {
                panic!("expected Binance control frame");
            };

            assert_eq!(event.error.is_some(), *has_nested_error);
        }
    }
}
