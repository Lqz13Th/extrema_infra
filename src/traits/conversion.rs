use std::future::ready;

use crate::errors::{InfraError, InfraResult};
use crate::task_execution::task_ws::{WsChannel, WsSubscription};

pub trait IntoWsData {
    type Output;
    fn into_ws(self) -> Self::Output;
}

pub trait WsSubscribe {
    fn ws_cex_pub_subscription(
        &self,
        _ws_channel: &WsChannel,
        _symbols: &[String]
    ) -> impl Future<Output = InfraResult<WsSubscription>> {
        ready(Err(InfraError::Unimplemented))
    }

    fn ws_cex_pri_subscription(
        &self,
        _ws_channel: &WsChannel,
    ) -> impl Future<Output = InfraResult<WsSubscription>> {
        ready(Err(InfraError::Unimplemented))
    }
}