use std::sync::Arc;
use serde::de::DeserializeOwned;
use serde_json::from_slice;
use futures_util::{SinkExt, StreamExt};
use tokio::{
    net::TcpStream,
    sync::{broadcast, mpsc},
    time::{
        error::Elapsed,
        sleep,
        timeout,
        Duration,
    },
};
use tungstenite::{
    protocol::Message,
    Bytes,
    Error,
};
use tokio_tungstenite::{
    connect_async,
    MaybeTlsStream,
    WebSocketStream,
};

use tracing::{error, info, warn};

use crate::errors::{InfraError, InfraResult};
use crate::arch::{
    market_assets::{
        exchange::{
            binance::{
                binance_ws_msg::BinanceWsData,
                schemas::um_futures_ws::{
                    agg_trades::WsAggTradeBinanceUM,
                    candles::WsCandleBinanceUM,
                }
            },
            okx::{
                okx_ws_msg::OkxWsData,
                schemas::ws::{
                    account_order::WsAccountOrderOkx,
                    trades::WsTradesOkx,
                },
            },
        },
        market_core::Market,
    },
    strategy_base::{
        command::{
            ack_handle::{AckHandle, AckStatus},
            command_core::TaskCommand,
        },
        handler::handler_core::*,
    },
    traits::conversion::IntoWsData,
};

use super::{
    task_general::LogLevel,
    task_ws::{WsChannel, WsTaskInfo}
};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
static PING: Bytes = Bytes::from_static(b"ping");

#[derive(Debug)]
pub(crate) struct WsTaskBuilder{
    pub cmd_rx: mpsc::Receiver<TaskCommand>,
    pub board_cast_channel: Arc<Vec<BoardCastChannel>>,
    pub ws_info: Arc<WsTaskInfo>,
    pub task_id: u64,
}

impl WsTaskBuilder {
    async fn connect_websocket(
        &self,
        url: &str,
    ) -> InfraResult<WsStream> {
        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| {
                error!("WebSocket connection failed: {:?}", e);
                InfraError::WebSocket(Box::new(e))
            })?;
        Ok(ws_stream)
    }
    async fn handle_ws_msg<WsData>(
        &mut self,
        msg: Result<Option<Result<Message, Error>>, Elapsed>,
        ws_stream: &mut WsStream,
        tx: &broadcast::Sender<InfraMsg<WsData::Output>>,
    ) -> bool
    where
        WsData: DeserializeOwned + IntoWsData + Send + 'static,
        WsData::Output: Send + Sync + 'static
    {
        match msg {
            Ok(Some(Ok(Message::Text(text)))) => {
                match from_slice::<WsData>(text.as_ref()) {
                    Ok(parsed_raw) => {
                        let _ = tx.send(InfraMsg {
                            task_id: self.task_id,
                            data: Arc::new(parsed_raw.into_ws()),
                        });
                    },
                    Err(e) => {
                        self.log(
                            LogLevel::Warn,
                            &format!("Failed to deserialize WS text: {}", e)
                        );
                    },
                };
            },
            Ok(Some(Ok(Message::Binary(bytes)))) => {
                match from_slice::<WsData>(bytes.as_ref()) {
                    Ok(parsed_raw) => {
                        let _ = tx.send(InfraMsg {
                            task_id: self.task_id,
                            data: Arc::new(parsed_raw.into_ws()),
                        });
                    },
                    Err(e) => {
                        self.log(
                            LogLevel::Warn,
                            &format!("Failed to deserialize WS binary: {:?}", e)
                        );
                    },
                };
            },
            Ok(Some(Ok(Message::Ping(payload)))) => {
                let _ = ws_stream.send(Message::Pong(payload)).await;
            },
            Ok(Some(Ok(Message::Close(frame)))) => {
                self.log(LogLevel::Error, &format!("WebSocket closed: {:?}", frame));
                return true;
            },
            Ok(Some(Err(e))) => {
                self.log(LogLevel::Error, &format!("Error receiving WS message: {:?}", e));
                return true;
            },
            Ok(None) => {
                self.log(LogLevel::Error, "WebSocket stream ended");
                return true;
            },
            Err(_) => {
                if let Err(e) = ws_stream.send(Message::Ping(PING.clone())).await {
                    self.log(LogLevel::Error, &format!("Failed to send ping: {:?}", e));
                    return true;
                }
            },
            _ => {},
        };

        false
    }

    async fn handle_command(
        &mut self,
        cmd: Option<TaskCommand>,
        ws_stream: &mut WsStream,
    ) -> bool {
        if let Some(cmd) = cmd {
            match cmd {
                TaskCommand::WsMessage { msg, ack } => {
                    self.send_cmd(ws_stream, msg, ack, AckStatus::WsMessage).await
                },
                TaskCommand::WsShutdown { msg, ack } => {
                    self.send_cmd(ws_stream, msg, ack, AckStatus::WsShutdown).await;
                    return true;
                },
                _ => {
                    self.log(
                        LogLevel::Warn,
                        &format!("Unexpected command, auto-ack: {:?}", cmd),
                    )
                },
            };
        }

        false
    }

    async fn send_cmd(
        &mut self,
        ws_stream: &mut WsStream,
        msg: String,
        ack_handle: AckHandle,
        ack_status: AckStatus,
    ) {
        if ws_stream.send(Message::text(msg.clone())).await.is_err() {
            self.log(
                LogLevel::Error,
                &format!("Failed to send {:?}: {}", ack_status, msg)
            );
        } else {
            self.log(LogLevel::Info,
                &format!("{:?}: {}", ack_status, msg)
            );
        }

        ack_handle.respond(ack_status);
    }


    async fn ws_loop<WsData>(
        &mut self,
        tx: broadcast::Sender<InfraMsg<WsData::Output>>,
        ws_stream: &mut WsStream,
    )
    where
        WsData: DeserializeOwned + IntoWsData + Send + 'static,
        WsData::Output: Send + Sync + 'static,
    {
        let timeout_sec = Duration::from_secs(10);

        loop {
            tokio::select! {
                msg = timeout(timeout_sec, ws_stream.next()) => {
                    if self.handle_ws_msg::<WsData>(msg, ws_stream, &tx).await {
                        break;
                    };
                },
                cmd = self.cmd_rx.recv() => {
                    if self.handle_command(cmd, ws_stream).await {
                        break;
                    }
                },
            }
        }
    }

    async fn ws_channel_distribution(
        &mut self,
        ws_stream: &mut WsStream,
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
            },
            (Market::Okx, WsChannel::AccountOrder) => {
                if let Some(tx) = find_acc_order(&self.board_cast_channel) {
                    self.ws_loop::<OkxWsData<WsAccountOrderOkx>>(
                        tx,
                        ws_stream
                    ).await;
                } else {
                    self.log(
                        LogLevel::Warn,
                        "No broadcast channel found for OKX Acc Order"
                    );
                }
            },
            (Market::Okx, WsChannel::Trades(..)) => {
                if let Some(tx) = find_trade(&self.board_cast_channel) {
                    self.ws_loop::<OkxWsData<WsTradesOkx>>(
                        tx,
                        ws_stream
                    ).await;
                } else {
                    self.log(
                        LogLevel::Warn,
                        "No broadcast channel found for OKX Trades"
                    );
                }
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
        if let Some(tx) = find_ws_event(&self.board_cast_channel) {
            let msg = InfraMsg {
                task_id: self.task_id,
                data: self.ws_info.clone(),
            };

            if let Err(e) = tx.send(msg) {
                self.log(LogLevel::Warn, &format!("CEX event send failed: {:?}", e));
            }
        } else {
            self.log(LogLevel::Warn, "No broadcast channel found for CEX event");
        }
    }

    pub(crate) async fn ws_mid_relay(&mut self) {
        let sleep_interval = Duration::from_secs(5 + 3 * self.task_id);
        self.log(LogLevel::Info, "Spawned ws task");

        loop {
            sleep(sleep_interval).await;
            self.ws_cex_event();
            self.log(LogLevel::Info, "Initiated");

            let initial_command = self.cmd_rx.recv().await;
            let (url, ack) = match initial_command {
                Some(TaskCommand::WsConnect { msg, ack }) => (msg, ack),
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

            let mut ws_stream = match self.connect_websocket(&url).await {
                Ok(ws) => ws,
                Err(e) => {
                    self.log(LogLevel::Error, &format!("Failed to connect ws: {:?}", e));
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            ack.respond(AckStatus::WsConnect);
            self.ws_channel_distribution(&mut ws_stream).await;

        }
    }

    fn log(&self, level: LogLevel, msg: &str) {
        match level {
            LogLevel::Info => {
                info!("Ws task: {:?}, task id: {}. {}", self.ws_info, self.task_id, msg)
            },
            LogLevel::Warn => {
                warn!("Ws task: {:?}, task id: {}. {}", self.ws_info, self.task_id, msg)
            },
            LogLevel::Error => {
                error!("Ws task: {:?}, task id: {}. {}", self.ws_info, self.task_id, msg)
            },
        }
    }
}

