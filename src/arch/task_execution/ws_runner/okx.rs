use crate::arch::{
    market_assets::exchange::okx::{
        okx_ws_msg::OkxWsData,
        schemas::ws::{
            account_bal_and_pos::WsBalAndPosOkx, account_order::WsAccountOrderOkx,
            account_position::WsAccountPositionOkx, candles::WsCandleOkx, lob::OkxWsLobBook,
            trades::WsTradesOkx,
        },
    },
    strategy_base::handler::task_channel::TaskEvent,
    task_execution::{task_general::LogLevel, task_ws::WsChannel},
};

use super::{WsStream, WsTaskRunner, ws_decode::decode_raw_ws};

impl WsTaskRunner {
    pub(super) async fn ws_channel_okx(&mut self, ws_stream: &mut WsStream) {
        match &self.ws_info.ws_channel {
            WsChannel::AccountOrders => {
                self.ws_loop(
                    TaskEvent::AccOrder,
                    ws_stream,
                    OkxWsData::<WsAccountOrderOkx>::decode_batch,
                )
                .await;
            },
            WsChannel::AccountBalAndPos => {
                self.ws_loop(
                    TaskEvent::AccBalPos,
                    ws_stream,
                    OkxWsData::<WsBalAndPosOkx>::decode_batch,
                )
                .await;
            },
            WsChannel::AccountPositions => {
                self.ws_loop(
                    TaskEvent::AccPos,
                    ws_stream,
                    OkxWsData::<WsAccountPositionOkx>::decode_batch,
                )
                .await;
            },
            WsChannel::Candles(..) => {
                self.ws_loop(
                    TaskEvent::Candle,
                    ws_stream,
                    OkxWsData::<WsCandleOkx>::decode_batch,
                )
                .await;
            },
            WsChannel::Trades(..) => {
                self.ws_loop(
                    TaskEvent::Trade,
                    ws_stream,
                    OkxWsData::<WsTradesOkx>::decode_batch,
                )
                .await;
            },
            WsChannel::Lob(..) => {
                self.ws_loop(
                    TaskEvent::Lob,
                    ws_stream,
                    OkxWsData::<OkxWsLobBook>::decode_batch,
                )
                .await;
            },
            WsChannel::Other(_) => {
                self.ws_loop(TaskEvent::WsOther, ws_stream, decode_raw_ws)
                    .await;
            },
            c => {
                self.log(LogLevel::Warn, &format!("Unknown Okx channel: {:?}", c));
            },
        };
    }
}
