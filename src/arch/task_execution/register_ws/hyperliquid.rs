use crate::arch::{
    market_assets::exchange::hyperliquid::{
        hyperliquid_ws_msg::HyperliquidWsData,
        schemas::ws::{
            account_order::WsAccountOrderHyperliquid,
            account_position::WsAccountPositionHyperliquid, trades::WsTradeHyperliquid,
        },
    },
    strategy_base::handler::handler_core::{find_acc_order, find_acc_pos, find_trade},
    task_execution::{task_general::LogLevel, task_ws::WsChannel},
};

use super::{WsStream, WsTaskBuilder};

impl WsTaskBuilder {
    pub(super) async fn ws_channel_hyperliquid(&mut self, ws_stream: &mut WsStream) {
        match &self.ws_info.ws_channel {
            WsChannel::Trades(..) => {
                if let Some(tx) = find_trade(&self.board_cast_channel) {
                    self.ws_loop::<HyperliquidWsData<WsTradeHyperliquid>>(tx, ws_stream)
                        .await;
                } else {
                    self.log(
                        LogLevel::Warn,
                        "No broadcast channel found for Hyperliquid Trades",
                    );
                }
            },
            WsChannel::AccountOrders => {
                if let Some(tx) = find_acc_order(&self.board_cast_channel) {
                    self.ws_loop::<HyperliquidWsData<WsAccountOrderHyperliquid>>(tx, ws_stream)
                        .await;
                } else {
                    self.log(
                        LogLevel::Warn,
                        "No broadcast channel found for Hyperliquid Acc Order",
                    );
                }
            },
            WsChannel::AccountPositions => {
                if let Some(tx) = find_acc_pos(&self.board_cast_channel) {
                    self.ws_loop::<HyperliquidWsData<WsAccountPositionHyperliquid>>(tx, ws_stream)
                        .await;
                } else {
                    self.log(
                        LogLevel::Warn,
                        "No broadcast channel found for Hyperliquid Acc Position",
                    );
                }
            },
            c => {
                self.log(
                    LogLevel::Warn,
                    &format!("Unknown Hyperliquid channel: {:?}", c),
                );
            },
        };
    }
}
