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
    task_execution::{
        task_general::LogLevel,
        task_ws::{LobParam, WsChannel},
    },
};

use super::{WsStream, WsTaskRunner, ws_decode::decode_raw_ws};

impl WsTaskRunner {
    pub(super) async fn ws_channel_hyperliquid(&mut self, ws_stream: &mut WsStream) {
        match &self.ws_info.ws_channel {
            WsChannel::AccountOrders => {
                self.ws_loop(
                    TaskEvent::AccOrder,
                    ws_stream,
                    HyperliquidWsData::<WsAccountOrderHyperliquid>::decode_batch,
                )
                .await;
            },
            WsChannel::AccountPositions => {
                self.ws_loop(
                    TaskEvent::AccPos,
                    ws_stream,
                    HyperliquidWsData::<WsAccountPositionHyperliquid>::decode_clearinghouse,
                )
                .await;
            },
            WsChannel::Trades(..) => {
                self.ws_loop(
                    TaskEvent::Trade,
                    ws_stream,
                    HyperliquidWsData::<WsTradeHyperliquid>::decode_batch,
                )
                .await;
            },
            WsChannel::Lob(Some(LobParam::Bbo { .. })) => {
                self.ws_loop(
                    TaskEvent::Lob,
                    ws_stream,
                    HyperliquidWsData::<WsLobHyperliquid>::decode_bbo,
                )
                .await;
            },
            WsChannel::Lob(_) => {
                self.ws_loop(
                    TaskEvent::Lob,
                    ws_stream,
                    HyperliquidWsData::<WsLobHyperliquid>::decode_l2_book,
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
                    &format!("Unknown Hyperliquid channel: {:?}", c),
                );
            },
        };
    }
}
