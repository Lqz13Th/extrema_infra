use std::sync::Arc;
use std::time::Duration;
use prost::bytes::Bytes;
use serde::de::DeserializeOwned;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, Mutex};
use tokio::time::{sleep, timeout};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tracing::{error, info, warn};
use tungstenite::Message;

use crate::errors::InfraError;
use crate::infra_core::env_core::EnvCore;
use crate::strategy_base::event_notify::board_cast_channels::{find_lob, find_timer, find_trade};
use crate::market_assets::base_data::Market;
use crate::task_execution::ws_register::{WsChannel, WsTaskBuilder};
use crate::traits::conversion::IntoWsData;
use crate::traits::strategy::Strategy;

#[derive(Debug, Clone)]
pub struct AltTaskInfo {
    pub alt_task_type: AltTaskType,
}

#[derive(Debug, Clone)]
pub enum AltTaskType {
    NeuralNetwork(u64),
    TimerBasedState(u64),
}


#[derive(Clone, Debug)]
pub(crate) struct AltTaskBuilder<S> {
    pub(crate) core: EnvCore<S>,
    pub(crate) alt_info: AltTaskInfo,
}


impl<S> AltTaskBuilder<S>
where
    S: Strategy + Clone
{
    async fn alt_task_distribution(&self) {
        match self.alt_info.alt_task_type {
            AltTaskType::NeuralNetwork(n) => {
                warn!("unimplemented, AltTaskType::NeuralNetwork({})", n);
            },
            AltTaskType::TimerBasedState(n) => {
                if let Some(tx) = find_timer(&self.core.board_cast_channels) {
                    loop {
                        sleep(Duration::from_millis(n)).await;
                        match tx.send(()) {
                            Ok(_) => {},
                            Err(broadcast::error::SendError(e)) => {
                                warn!("Timer send failed: {:?}", e)
                            },
                        };
                    }
                } else {
                    warn!("No broadcast channel found for TimerBasedState");
                }
            },
        };
    }

    pub(crate) async fn alt_mid_relay(
        &mut self,
    ) {
        let sleep_interval = Duration::from_secs(5);
        loop {
            sleep(sleep_interval).await;
            info!(
                "State management task start, Manager Type: {:?}",
                self.alt_info,
            );

            self.alt_task_distribution().await;
        }
    }
}