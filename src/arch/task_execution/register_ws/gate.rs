use crate::arch::{
    market_assets::exchange::gate::{
        gate_ws_msg::GateWsData,
        schemas::futures_ws::{
            account_order::WsAccountOrderGateFutures,
            account_position::WsAccountPositionGateFutures, candles::WsCandleGateFutures,
            trades::WsTradeGateFutures,
        },
        schemas::spot_ws::account_order::WsAccountOrderGateSpot,
    },
    strategy_base::handler::handler_core::{find_acc_order, find_acc_pos, find_candle, find_trade},
    task_execution::{task_general::LogLevel, task_ws::WsChannel},
};

use super::{WsStream, WsTaskBuilder};

impl WsTaskBuilder {
    pub(super) async fn ws_channel_gate_futures(&mut self, ws_stream: &mut WsStream) {
        match &self.ws_info.ws_channel {
            WsChannel::Trades(..) => {
                if let Some(tx) = find_trade(&self.board_cast_channel) {
                    self.ws_loop::<GateWsData<WsTradeGateFutures>>(tx, ws_stream)
                        .await;
                } else {
                    self.log(
                        LogLevel::Warn,
                        "No broadcast channel found for Gate Futures Trades",
                    );
                }
            },
            WsChannel::Candles(..) => {
                if let Some(tx) = find_candle(&self.board_cast_channel) {
                    self.ws_loop::<GateWsData<WsCandleGateFutures>>(tx, ws_stream)
                        .await;
                } else {
                    self.log(
                        LogLevel::Warn,
                        "No broadcast channel found for Gate Futures Candles",
                    );
                }
            },
            WsChannel::AccountOrders => {
                if let Some(tx) = find_acc_order(&self.board_cast_channel) {
                    self.ws_loop::<GateWsData<WsAccountOrderGateFutures>>(tx, ws_stream)
                        .await;
                } else {
                    self.log(
                        LogLevel::Warn,
                        "No broadcast channel found for Gate Futures Acc Order",
                    );
                }
            },
            WsChannel::AccountPositions => {
                if let Some(tx) = find_acc_pos(&self.board_cast_channel) {
                    self.ws_loop::<GateWsData<WsAccountPositionGateFutures>>(tx, ws_stream)
                        .await;
                } else {
                    self.log(
                        LogLevel::Warn,
                        "No broadcast channel found for Gate Futures Acc Position",
                    );
                }
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
                if let Some(tx) = find_acc_order(&self.board_cast_channel) {
                    self.ws_loop::<GateWsData<WsAccountOrderGateSpot>>(tx, ws_stream)
                        .await;
                } else {
                    self.log(
                        LogLevel::Warn,
                        "No broadcast channel found for Gate Spot Acc Order",
                    );
                }
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
