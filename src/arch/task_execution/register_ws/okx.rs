use crate::arch::{
    market_assets::exchange::okx::{
        okx_ws_msg::OkxWsData,
        schemas::ws::{
            account_bal_and_pos::WsBalAndPosOkx, account_order::WsAccountOrderOkx,
            account_position::WsAccountPositionOkx, lob::OkxWsLobBook, trades::WsTradesOkx,
        },
    },
    strategy_base::handler::handler_core::{
        find_acc_bal_pos, find_acc_order, find_acc_pos, find_lob, find_trade,
    },
    task_execution::{task_general::LogLevel, task_ws::WsChannel},
};

use super::{WsStream, WsTaskBuilder};

impl WsTaskBuilder {
    pub(super) async fn ws_channel_okx(&mut self, ws_stream: &mut WsStream) {
        match &self.ws_info.ws_channel {
            WsChannel::Trades(..) => {
                if let Some(tx) = find_trade(&self.board_cast_channel) {
                    self.ws_loop::<OkxWsData<WsTradesOkx>>(tx, ws_stream).await;
                } else {
                    self.log(LogLevel::Warn, "No broadcast channel found for Okx Trades");
                }
            },
            WsChannel::Lob(..) => {
                if let Some(tx) = find_lob(&self.board_cast_channel) {
                    self.ws_loop::<OkxWsData<OkxWsLobBook>>(tx, ws_stream).await;
                } else {
                    self.log(LogLevel::Warn, "No broadcast channel found for Okx LOB");
                }
            },
            WsChannel::AccountOrders => {
                if let Some(tx) = find_acc_order(&self.board_cast_channel) {
                    self.ws_loop::<OkxWsData<WsAccountOrderOkx>>(tx, ws_stream)
                        .await;
                } else {
                    self.log(
                        LogLevel::Warn,
                        "No broadcast channel found for Okx Acc Order",
                    );
                }
            },
            WsChannel::AccountPositions => {
                if let Some(tx) = find_acc_pos(&self.board_cast_channel) {
                    self.ws_loop::<OkxWsData<WsAccountPositionOkx>>(tx, ws_stream)
                        .await;
                } else {
                    self.log(
                        LogLevel::Warn,
                        "No broadcast channel found for Okx Acc Position",
                    );
                }
            },
            WsChannel::AccountBalAndPos => {
                if let Some(tx) = find_acc_bal_pos(&self.board_cast_channel) {
                    self.ws_loop::<OkxWsData<WsBalAndPosOkx>>(tx, ws_stream)
                        .await;
                } else {
                    self.log(
                        LogLevel::Warn,
                        "No broadcast channel found for Okx Acc Bal and Pos",
                    );
                }
            },
            c => {
                self.log(LogLevel::Warn, &format!("Unknown Okx channel: {:?}", c));
            },
        };
    }
}
