use crate::strategy_base::event_notify::board_cast_channels::BoardCastChannel::Lob;
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
use tokio::sync::broadcast;
use tracing::{info, warn, error};

use crate::errors::InfraError;
use crate::infra_core::env_core::EnvCore;
use crate::traits::{
    conversion::*,
};
use crate::strategy_base::event_notify::board_cast_channels::*;
use crate::strategy_base::event_notify::cex_notify::*;
use crate::market_assets::base_data::Market;
use crate::traits::strategy::Strategy;
use crate::market_assets::cex::{
    binance::binance_um_futures_cli::*,
    binance::um_futures_ws::agg_trades::*
};

#[derive(Debug, Clone)]
pub struct WsTaskInfo {
    pub market: Market,
    pub channel: WsChannel,
    pub market_cex: bool,
    pub public_uri: bool,
    pub chunk: usize,
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


type WsWrite = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
pub type SharedWsWrite = Arc<Mutex<WsWrite>>;

#[derive(Debug, Clone)]
pub struct WsTaskHandle {
    pub shared_ws_write: SharedWsWrite,
    pub channel: WsChannel,
    pub task_numb: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct WsTaskBuilder<S> {
    pub(crate) core: EnvCore<S>,
    pub(crate) ws_info: WsTaskInfo,
    pub(crate) task_numb: usize,
}

impl<S> WsTaskBuilder<S>
where
    S: Strategy + Clone
{
    async fn handle_initial_ws_request(
        &self,
    ) -> Result<(Option<String>, String), InfraError> {
        match (&self.ws_info.market_cex, &self.ws_info.public_uri) {
            (true, true) => {
                // let binance_um = BinanceUM::new();
                // let (msg, url) = binance_um.ws_cex_pub_subscription(
                //     &self.ws_info.channel,
                //     &tokens,
                // )?;
                // 
                // Ok((msg, url))
                todo!()
            },
            (true, false) => {
                // Ok(self.market.handle_ws_cex_pri_subscription(
                //     &self.channel
                // ).await?)
                todo!()
            },
            _ => {
                error!("WsTaskInfo::handle_request() called on unhandled ws request");
                Err(InfraError::UnknownWsSubscription)
            },
        }
    }

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
        WsData: DeserializeOwned + IntoWsData + Send + 'static,
        WsData::Output: Send + Sync + 'static,
    {
        let (ws_write, mut ws_read) = ws_stream.split();
        let ws_write_arc = Arc::new(Mutex::new(ws_write));
        let task_handle = WsTaskHandle {
            shared_ws_write: ws_write_arc.clone(),
            channel: self.ws_info.channel.clone(),
            task_numb: self.task_numb
        };
        
        self.core.strategies.on_ws_init(task_handle).await;

        loop {
            match timeout(Duration::from_secs(13), ws_read.next()).await {
                Ok(Some(Ok(Message::Text(text)))) => {
                    match serde_json::from_slice::<WsData>(text.as_bytes()) {
                        Ok(parsed_raw) => {
                            let parsed = parsed_raw.into_ws();
                            let _ = tx.send(Arc::new(parsed));
                        },
                        Err(e) => {
                            error!("Failed to deserialize message: {}. Error: {:?}", text, e);
                        },
                    };
                },
                Ok(Some(Ok(Message::Ping(payload)))) => {
                    if let Err(e) = ws_write_arc
                        .lock().await
                        .send(Message::Ping(payload)).await {
                        error!("Error sending Pong: {:?}", e);
                    }
                },
                Ok(Some(Ok(Message::Pong(_msg)))) => {
                },
                Ok(Some(Ok(Message::Close(frame)))) => {
                    error!("WebSocket closed: {:?}", frame);
                    break;
                },
                Ok(Some(Err(e))) => {
                    error!("Error receiving message: {:?}", e);
                    break;
                },
                Ok(None) => {
                    error!("Connection closed!");
                    break;
                },
                Err(_e) => {
                    if ws_write_arc
                        .lock().await
                        .send(Message::Ping(Bytes::from("ping"))).await
                        .is_err() {
                        error!("Error sending ping");
                        break;
                    }
                },
                _ => {},
            };
        }
    }

    pub(crate) async fn ws_channel_distribution(
        &mut self,
        ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) {
        match (&self.ws_info.market, &self.ws_info.channel) {
            (Market::BinanceUmFutures, WsChannel::Trades(..)) => {
                if let Some(tx) = find_trade(&self.core.board_cast_channels) {
                    self.ws_loop::<WsAggTradeBinanceUM>(tx, ws_stream).await;
                } else {
                    warn!("No broadcast channel found for Binance Futures Trades");
                }
            },
            (Market::BinanceUmFutures, WsChannel::Lob) => {
                todo!()
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

    pub(crate) async fn ws_mid_relay(
        &mut self,
    ) {
        let sleep_interval = Duration::from_secs(5 + 3 * self.task_numb as u64);
        loop {
            sleep(sleep_interval).await;
            info!("Websocket task start: {}", self.task_numb);

            let (msg, url) = match self.handle_initial_ws_request().await {
                Ok((msg, url)) => {
                    (msg, url)
                },
                Err(e) => {
                    error!("Task: {} failed to handle request: {:?}", self.task_numb, e);
                    continue;
                },
            };

            let mut ws_stream = match self.connect_websocket(&url).await {
                Ok(ws_stream) => ws_stream,
                Err(e) => {
                    error!("Task: {} failed to connect WebSocket: {:?}", self.task_numb, e);
                    sleep(Duration::from_secs(5)).await;
                    continue;
                },
            };

            if let Some(msg) = msg {
                if ws_stream.send(Message::text(msg)).await.is_err() {
                    error!("Task: {} failed to send message", self.task_numb);
                    continue;
                }
            }

            self.ws_channel_distribution(ws_stream).await;
            sleep(Duration::from_secs(5)).await;
        }
    }
}
