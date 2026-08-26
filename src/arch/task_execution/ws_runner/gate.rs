use crate::arch::{
    market_assets::exchange::gate::{
        gate_ws_msg::GateWsData,
        schemas::futures_ws::{
            account_order::WsAccountOrderGateFutures,
            account_position::WsAccountPositionGateFutures,
            candles::WsCandleGateFutures,
            lob::{WsBookTickerGateFutures, WsOrderBookGateFutures, WsOrderBookUpdateGateFutures},
            trades::WsTradeGateFutures,
        },
        schemas::spot_ws::account_order::WsAccountOrderGateSpot,
    },
    strategy_base::handler::task_channel::TaskEvent,
    task_execution::{
        task_general::LogLevel,
        task_ws::{LobParam, WsChannel},
    },
};

use super::{WsStream, WsTaskRunner, ws_decode::decode_raw_ws};

impl WsTaskRunner {
    pub(super) async fn ws_channel_gate_futures(&mut self, ws_stream: &mut WsStream) {
        match &self.ws_info.ws_channel {
            WsChannel::AccountOrders => {
                self.ws_loop(
                    TaskEvent::AccOrder,
                    ws_stream,
                    GateWsData::<WsAccountOrderGateFutures>::decode_batch,
                )
                .await;
            },
            WsChannel::AccountPositions => {
                self.ws_loop(
                    TaskEvent::AccPos,
                    ws_stream,
                    GateWsData::<WsAccountPositionGateFutures>::decode_batch,
                )
                .await;
            },
            WsChannel::Candles(..) => {
                self.ws_loop(
                    TaskEvent::Candle,
                    ws_stream,
                    GateWsData::<WsCandleGateFutures>::decode_batch,
                )
                .await;
            },
            WsChannel::Trades(..) => {
                self.ws_loop(
                    TaskEvent::Trade,
                    ws_stream,
                    GateWsData::<WsTradeGateFutures>::decode_batch,
                )
                .await;
            },
            WsChannel::Lob(lob_param) => match lob_param {
                Some(LobParam::Bbo { .. }) => {
                    self.ws_loop(
                        TaskEvent::Lob,
                        ws_stream,
                        GateWsData::<WsBookTickerGateFutures>::decode_single,
                    )
                    .await;
                },
                Some(LobParam::Snapshot { .. }) => {
                    self.ws_loop(
                        TaskEvent::Lob,
                        ws_stream,
                        GateWsData::<WsOrderBookGateFutures>::decode_single,
                    )
                    .await;
                },
                None | Some(LobParam::Incremental { .. }) => {
                    self.ws_loop(
                        TaskEvent::Lob,
                        ws_stream,
                        GateWsData::<WsOrderBookUpdateGateFutures>::decode_single,
                    )
                    .await;
                },
            },
            WsChannel::Other(_) => {
                self.ws_loop(TaskEvent::WsOther, ws_stream, decode_raw_ws)
                    .await;
            },
            c => {
                self.log(
                    LogLevel::Warn,
                    &format!("Unknown Gate Futures channel: {:?}", c),
                );
            },
        };
    }

    pub(super) async fn ws_channel_gate_spot(&mut self, ws_stream: &mut WsStream) {
        match &self.ws_info.ws_channel {
            WsChannel::AccountOrders => {
                self.ws_loop(
                    TaskEvent::AccOrder,
                    ws_stream,
                    GateWsData::<WsAccountOrderGateSpot>::decode_batch,
                )
                .await;
            },
            WsChannel::Other(_) => {
                self.ws_loop(TaskEvent::WsOther, ws_stream, decode_raw_ws)
                    .await;
            },
            c => {
                self.log(
                    LogLevel::Warn,
                    &format!("Unknown Gate Spot channel: {:?}", c),
                );
            },
        };
    }
}
