use crate::arch::{
    market_assets::exchange::hyperliquid::{
        hyperliquid_ws_msg::HyperliquidWsData,
        schemas::ws::{
            account_order::WsAccountOrderHyperliquid,
            account_position::WsAccountPositionHyperliquid, lob::WsLobHyperliquid,
            trades::WsTradeHyperliquid,
        },
    },
    strategy_base::handler::task_channel::TaskEvent,
    task_execution::{task_general::LogLevel, task_ws::WsChannel},
};

use super::{WsStream, WsTaskBuilder};

impl WsTaskBuilder {
    pub(super) async fn ws_channel_hyperliquid(&mut self, ws_stream: &mut WsStream) {
        match &self.ws_info.ws_channel {
            WsChannel::AccountOrders => {
                self.ws_loop::<HyperliquidWsData<WsAccountOrderHyperliquid>>(
                    TaskEvent::AccOrder,
                    ws_stream,
                )
                .await;
            },
            WsChannel::AccountPositions => {
                self.ws_loop::<HyperliquidWsData<WsAccountPositionHyperliquid>>(
                    TaskEvent::AccPos,
                    ws_stream,
                )
                .await;
            },
            WsChannel::Trades(..) => {
                self.ws_loop::<HyperliquidWsData<WsTradeHyperliquid>>(TaskEvent::Trade, ws_stream)
                    .await;
            },
            WsChannel::Lob(..) => {
                self.ws_loop::<HyperliquidWsData<WsLobHyperliquid>>(TaskEvent::Lob, ws_stream)
                    .await;
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
