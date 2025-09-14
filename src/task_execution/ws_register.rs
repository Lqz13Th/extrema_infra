use std::sync::Arc;
use serde::de::DeserializeOwned;
use tokio::{
    sync::Mutex,
    net::TcpStream,
    time::{
        sleep,
        timeout,
        Duration,
    },
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        Bytes,
        protocol::Message,
    },
    MaybeTlsStream,
    WebSocketStream
};
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::SplitSink;
use tokio::sync::{broadcast, mpsc};
use tracing::{info, warn, error};

use crate::errors::InfraError;
use crate::infra_core::env_core::EnvCore;
use crate::traits::{
    conversion::*,
};
use crate::strategy_base::handler::handler_core::*;
use crate::strategy_base::handler::cex_events::*;
use crate::market_assets::market_core::Market;
use crate::traits::strategy::Strategy;
use crate::market_assets::cex::{
    binance::binance_um_futures_cli::*,
    binance::um_futures_ws::agg_trades::*
};
use crate::market_assets::cex::binance::um_futures_ws::candles::WsCandleBinanceUM;
use crate::strategy_base::command::command_core::TaskCommand;

#[derive(Debug, Clone)]
pub struct WsTaskInfo {
    pub market: Market,
    pub channel: WsChannel,
    pub chunk: usize,
}

#[derive(Debug, Clone)]
pub struct WsSubscription {
    pub msg: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone)]
pub enum WsChannel {
    Account,
    Candle(Option<CandleParam>),
    Trades(Option<TradesParam>),
    Tick,
    Lob,
    Other(String),
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub enum CandleParam {
    OneSecond,
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    OneHour,
    FourHours,
    OneDay,
    OneWeek,
}

#[derive(Debug, Clone)]
pub enum TradesParam {
    AggTrades,
    Trades,
}


#[derive(Debug)]
pub(crate) struct WsTaskBuilder{
    pub(crate) cmd_rx: mpsc::Receiver<TaskCommand>,
    pub(crate) channel: Arc<Vec<BoardCastChannel>>,
    pub(crate) ws_info: Arc<WsTaskInfo>,
    pub(crate) task_numb: usize,
}

impl WsTaskBuilder {

    async fn connect_websocket(
        &self,
        url: &str,
    ) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, InfraError> {
        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| {
                error!("WebSocket connection failed: {:?}", e);
                InfraError::WebSocket(e)
            })?;
        Ok(ws_stream)
    }


    async fn ws_loop<WsData>(
        &mut self,
        tx: broadcast::Sender<Arc<WsData::Output>>,
        ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) where
        WsData: DeserializeOwned + IntoWsData + Send + 'static + std::fmt::Debug,
        WsData::Output: Send + Sync + 'static,
    {
        let (mut ws_write, mut ws_read) = ws_stream.split();
        info!("start");
        loop {
            tokio::select! {
            msg = ws_read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                            info!("Received message: {:?}", text);
                        if let Ok(parsed_raw) = serde_json::from_slice::<WsData>(text.as_bytes()) {
                            info!("{:?}", parsed_raw);
                            let parsed = parsed_raw.into_ws();
                            let _ = tx.send(Arc::new(parsed));

                        } else {
                            warn!("Failed to deserialize WS message: {}", text);
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        let _ = ws_write.send(Message::Pong(payload)).await;
                    }
                    Some(Ok(Message::Close(frame))) => {
                        error!("WebSocket closed: {:?}", frame);
                        break;
                    }
                    Some(Err(e)) => {
                        error!("Error receiving WS message: {:?}", e);
                        break;
                    }
                    None => {
                        error!("WebSocket stream ended");
                        break;
                    }
                    _ => {}
                }
            },
            cmd = self.cmd_rx.recv() => {
                match cmd {
                    Some(TaskCommand::Subscribe { msg, ack }) => {
                        info!("Task {} subscribing: {}", self.task_numb, msg);
                        if ws_write.send(Message::text(msg)).await.is_err() {
                            error!("Failed to send subscribe message for {}", self.task_numb);
                        }
                        ack.respond(Ok(()));
                    },
                    Some(TaskCommand::Unsubscribe { msg, ack }) => {
                        info!("Task {} unsubscribing symbol: {}", self.task_numb, msg);
                        if ws_write.send(Message::text(msg)).await.is_err() {
                            error!("Failed to send unsubscribe message for {}", self.task_numb);
                        }
                        ack.respond(Ok(()));
                    },
                    Some(TaskCommand::Shutdown { msg, ack }) => {
                        info!("Task {} shutting down", self.task_numb);
                        ack.respond(Ok(()));
                        break;
                    },
                    Some(TaskCommand::NNInput(_input)) => {
                        todo!()
                    },
                    Some(TaskCommand::NNOutput(_output)) => {
                        todo!()
                    },
                    None => {
                        warn!("Command channel closed");
                        break;
                    },
                    Some(TaskCommand::Connect { .. }) => todo!(),
                }
            }
        }
        }
    }

    pub(crate) async fn ws_channel_distribution(
        &mut self,
        ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) {
        match (&self.ws_info.market, &self.ws_info.channel) {
            (Market::BinanceUmFutures, WsChannel::Trades(..)) => {
                if let Some(tx) = find_trade(&self.channel) {
                    self.ws_loop::<WsAggTradeBinanceUM>(tx, ws_stream).await;
                } else {
                    warn!("No broadcast channel found for Binance Futures Trades");
                }
            },
            (Market::BinanceUmFutures, WsChannel::Candle(..)) => {
                if let Some(tx) = find_candle(&self.channel) {
                    self.ws_loop::<WsCandleBinanceUM>(tx, ws_stream).await;
                } else {
                    warn!("No broadcast channel found for Binance Futures Candles");
                }
                // if let Some(tx) = find_lob(&self.core.board_cast_channels) {
                //     self.ws_loop::<WsLobBinance>(tx, ws_stream).await;
                // } else {
                //     warn!("No broadcast channel found for Binance Futures LOB");
                // }
            },
            (Market::SolPumpFun, WsChannel::Other(..)) => {

            },
            (market, _) => {
                warn!("Unknown channel for the market: {:?}", market)
            },
        };
    }

    pub(crate) async fn ws_mid_relay(&mut self) {
        let sleep_interval = Duration::from_secs(5 + 3 * self.task_numb as u64);

        loop {
            sleep(sleep_interval).await;
            info!("WebSocket task start: {}", self.task_numb);

            let initial_command = self.cmd_rx.recv().await;
            let (url, ack) = match initial_command {
                Some(TaskCommand::Connect { msg, ack }) => (msg, ack),
                Some(cmd) => {
                    warn!("Task {} received unexpected initial command: {:?}", self.task_numb, cmd);
                    continue;
                }
                None => {
                    warn!("Task {} command channel closed during init", self.task_numb);
                    break;
                }
            };


            let ws_stream = match self.connect_websocket(&url).await {
                Ok(ws) => ws,
                Err(e) => {
                    error!("Task {} failed to connect WebSocket: {:?}", self.task_numb, e);
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            ack.respond(Ok(()));
            info!("Task {} connected WS: {}", self.task_numb, url);

            self.ws_channel_distribution(ws_stream).await;
            sleep(Duration::from_secs(5)).await;
        }
    }

}
