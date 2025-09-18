use std::sync::Arc;
use serde_json::from_slice;
use serde::de::DeserializeOwned;
use futures_util::{SinkExt, StreamExt};
use tokio::{
    sync::{broadcast, mpsc},
    time::{sleep, Duration},
    net::TcpStream,
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::protocol::Message,
    MaybeTlsStream,
    WebSocketStream,
};

use tracing::{info, warn, error, debug};
use crate::errors::{InfraError, InfraResult};
use crate::market_assets::{
    market_core::Market,
    cex::{
        binance::{
            binance_ws_msg::BinanceWsData,
            um_futures_ws::{
                agg_trades::WsAggTradeBinanceUM,
                candles::WsCandleBinanceUM,
            }
        }
    }
};
use crate::strategy_base::{
    command::{
        ack_handle::AckStatus,
        command_core::TaskCommand
    },
    handler::handler_core::*,
};
use crate::traits::conversion::IntoWsData;

use super::{
    task_general::LogLevel,
    task_ws::{WsTaskInfo, WsChannel}
};


#[derive(Debug)]
pub(crate) struct WsTaskBuilder{
    pub cmd_rx: mpsc::Receiver<TaskCommand>,
    pub board_cast_channel: Arc<Vec<BoardCastChannel>>,
    pub ws_info: Arc<WsTaskInfo>,
    pub task_numb: u64,
}

impl WsTaskBuilder {
    async fn connect_websocket(
        &self,
        url: &str,
    ) -> InfraResult<WebSocketStream<MaybeTlsStream<TcpStream>>> {
        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| {
                error!("WebSocket connection failed: {:?}", e);
                InfraError::WebSocket(Box::new(e))
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
        loop {
            tokio::select! {
                msg = ws_read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            debug!(text = %text, "Received message");
                            if let Ok(parsed_raw) = from_slice::<WsData>(text.as_bytes()) {
                                let parsed = parsed_raw.into_ws();
                                let _ = tx.send(Arc::new(parsed));

                            } else {
                                self.log(
                                    LogLevel::Warn,
                                    &format!("Failed to deserialize WS message: {}", text)
                                );
                            }
                        },
                        Some(Ok(Message::Ping(payload))) => {
                            let _ = ws_write.send(Message::Pong(payload)).await;
                        },
                        Some(Ok(Message::Close(frame))) => {
                            self.log(LogLevel::Error, &format!("WebSocket closed: {:?}", frame));
                            break;
                        },
                        Some(Err(e)) => {
                            self.log(
                                LogLevel::Error,
                                &format!("Error receiving WS message: {:?}", e)
                            );
                            break;
                        },
                        None => {
                            self.log(LogLevel::Error, "WebSocket stream ended");
                            break;
                        },
                        _ => {},
                    };
                },
                cmd = self.cmd_rx.recv() => {
                    match cmd {
                        Some(TaskCommand::Login { msg, ack }) => {
                            if ws_write.send(Message::text(msg.clone())).await.is_err() {
                                self.log(
                                    LogLevel::Error,
                                    &format!("Failed to send login: {}", msg)
                                );
                            } else {
                                self.log(LogLevel::Info,&format!("Login: {}", msg));
                            }
                            ack.respond(Ok(AckStatus::Login));
                        },
                        Some(TaskCommand::Subscribe { msg, ack }) => {
                            if ws_write.send(Message::text(msg.clone())).await.is_err() {
                                self.log(
                                    LogLevel::Error,
                                    &format!("Failed to send subscribe: {}", msg)
                                );
                            } else {
                                self.log(LogLevel::Info,&format!("Subscribed: {}", msg));
                            }
                            ack.respond(Ok(AckStatus::Subscribe));
                        },
                        Some(TaskCommand::Unsubscribe { msg, ack }) => {
                            if ws_write.send(Message::text(msg.clone())).await.is_err() {
                                self.log(
                                    LogLevel::Error,
                                    &format!("Failed to send unsubscribe: {}", msg)
                                );
                            } else {
                                self.log(LogLevel::Info, &format!("Unsubscribed: {}", msg));
                            }
                            ack.respond(Ok(AckStatus::Unsubscribe));
                        },
                        Some(TaskCommand::Shutdown { msg, ack }) => {
                            self.log(LogLevel::Warn, &format!("Shutting down: {}", msg));
                            ack.respond(Ok(AckStatus::Shutdown));
                            break;
                        },
                        _ => self.log(LogLevel::Warn, "Unexpected command"),
                    };
                }
            }
        }
    }

    async fn ws_channel_distribution(
        &mut self,
        ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) {
        match (&self.ws_info.market, &self.ws_info.ws_channel) {
            (Market::BinanceUmFutures, WsChannel::Trades(..)) => {
                if let Some(tx) = find_trade(&self.board_cast_channel) {
                    self.ws_loop::<BinanceWsData<WsAggTradeBinanceUM>>(
                        tx,
                        ws_stream
                    ).await;
                } else {
                    self.log(
                        LogLevel::Warn,
                        "No broadcast channel found for Binance UmFutures Trades"
                    );
                }
            },
            (Market::BinanceUmFutures, WsChannel::Candle(..)) => {
                if let Some(tx) = find_candle(&self.board_cast_channel) {
                    self.ws_loop::<BinanceWsData<WsCandleBinanceUM>>(
                        tx,
                        ws_stream
                    ).await;
                } else {
                    self.log(
                        LogLevel::Warn,
                        "No broadcast channel found for Binance UmFutures Candles"
                    );
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
                self.log(
                    LogLevel::Warn,
                    &format!("Unknown channel for the market: {:?}", market)
                );
            },
        };
    }

    fn ws_cex_event(&self) {
        if let Some(tx) = find_cex_event(&self.board_cast_channel) {
            if let Err(e) = tx.send(self.ws_info.clone()) {
                self.log(LogLevel::Warn, &format!("CEX event send failed: {:?}", e));
            }
        } else {
            self.log(LogLevel::Warn, "No broadcast channel found for CEX event");
        }
    }

    pub(crate) async fn ws_mid_relay(&mut self) {
        let sleep_interval = Duration::from_secs(5 + 3 * self.task_numb);
        self.log(LogLevel::Info, "Spawned rest task");

        loop {
            sleep(sleep_interval).await;
            self.ws_cex_event();

            self.log(LogLevel::Info, "Initiated");

            let initial_command = self.cmd_rx.recv().await;
            let (url, ack) = match initial_command {
                Some(TaskCommand::Connect { msg, ack }) => (msg, ack),
                Some(cmd) => {
                    self.log(
                        LogLevel::Warn,
                        &format!("Received unexpected initial command: {:?}", cmd)
                    );
                    continue;
                },
                None => {
                    self.log(LogLevel::Warn, "Command channel closed during init");
                    continue;
                },
            };

            let ws_stream = match self.connect_websocket(&url).await {
                Ok(ws) => ws,
                Err(e) => {
                    self.log(LogLevel::Error, &format!("Failed to connect ws: {:?}", e));
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            ack.respond(Ok(AckStatus::Connect));
            self.ws_channel_distribution(ws_stream).await;

        }
    }

    fn log(&self, level: LogLevel, msg: &str) {
        match level {
            LogLevel::Info => {
                info!("Ws task: {:?}, task numb: {}. {}", self.ws_info, self.task_numb, msg)
            },
            LogLevel::Warn => {
                warn!("Ws task: {:?}, task numb: {}. {}", self.ws_info, self.task_numb, msg)
            },
            LogLevel::Error => {
                error!("Ws task: {:?}, task numb: {}. {}", self.ws_info, self.task_numb, msg)
            },
        }
    }
}

