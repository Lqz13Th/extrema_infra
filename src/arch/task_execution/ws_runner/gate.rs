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

use super::{WsStream, WsTaskRunner};

impl WsTaskRunner {
    pub(super) async fn ws_channel_gate_futures(&mut self, ws_stream: &mut WsStream) {
        match &self.ws_info.ws_channel {
            WsChannel::AccountOrders => {
                self.ws_loop::<GateWsData<WsAccountOrderGateFutures>>(
                    TaskEvent::AccOrder,
                    ws_stream,
                )
                .await;
            },
            WsChannel::AccountPositions => {
                self.ws_loop::<GateWsData<WsAccountPositionGateFutures>>(
                    TaskEvent::AccPos,
                    ws_stream,
                )
                .await;
            },
            WsChannel::Candles(..) => {
                self.ws_loop::<GateWsData<WsCandleGateFutures>>(TaskEvent::Candle, ws_stream)
                    .await;
            },
            WsChannel::Trades(..) => {
                self.ws_loop::<GateWsData<WsTradeGateFutures>>(TaskEvent::Trade, ws_stream)
                    .await;
            },
            WsChannel::Lob(lob_param) => match lob_param {
                Some(LobParam::Bbo { .. }) => {
                    self.ws_loop::<GateWsData<WsBookTickerGateFutures>>(TaskEvent::Lob, ws_stream)
                        .await;
                },
                Some(LobParam::Snapshot { .. }) => {
                    self.ws_loop::<GateWsData<WsOrderBookGateFutures>>(TaskEvent::Lob, ws_stream)
                        .await;
                },
                None | Some(LobParam::Incremental { .. }) => {
                    self.ws_loop::<GateWsData<WsOrderBookUpdateGateFutures>>(
                        TaskEvent::Lob,
                        ws_stream,
                    )
                    .await;
                },
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
                self.ws_loop::<GateWsData<WsAccountOrderGateSpot>>(TaskEvent::AccOrder, ws_stream)
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
