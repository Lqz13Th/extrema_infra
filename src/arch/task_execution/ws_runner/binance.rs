use crate::arch::{
    market_assets::exchange::binance::{
        binance_ws_msg::BinanceWsData,
        schemas::{
            cm_futures_ws::lob::{
                WsBookTickerBinanceCM, WsDiffDepthBinanceCM, WsPartialDepthBinanceCM,
            },
            spot_ws::account_order::WsAccountOrderEnvelopeBinanceSpot,
            um_futures_ws::{
                account_bal_and_pos::WsBalAndPosBinanceUM,
                account_order::WsAccountOrderBinanceUM,
                account_position::WsAccountPositionBinanceUM,
                agg_trades::WsAggTradeBinanceUM,
                candles::WsCandleBinanceUM,
                lob::{WsBookTickerBinanceUM, WsDiffDepthBinanceUM, WsPartialDepthBinanceUM},
            },
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
    pub(super) async fn ws_channel_binance_um(&mut self, ws_stream: &mut WsStream) {
        match &self.ws_info.ws_channel {
            WsChannel::AccountOrders => {
                self.ws_loop(
                    TaskEvent::AccOrder,
                    ws_stream,
                    BinanceWsData::<WsAccountOrderBinanceUM>::decode_single,
                )
                .await;
            },
            WsChannel::AccountBalAndPos => {
                self.ws_loop(
                    TaskEvent::AccBalPos,
                    ws_stream,
                    BinanceWsData::<WsBalAndPosBinanceUM>::decode_single,
                )
                .await;
            },
            WsChannel::AccountPositions => {
                self.ws_loop(
                    TaskEvent::AccPos,
                    ws_stream,
                    BinanceWsData::<WsAccountPositionBinanceUM>::decode_account_positions,
                )
                .await;
            },
            WsChannel::Candles(..) => {
                self.ws_loop(
                    TaskEvent::Candle,
                    ws_stream,
                    BinanceWsData::<WsCandleBinanceUM>::decode_single,
                )
                .await;
            },
            WsChannel::Trades(..) => {
                self.ws_loop(
                    TaskEvent::Trade,
                    ws_stream,
                    BinanceWsData::<WsAggTradeBinanceUM>::decode_single,
                )
                .await;
            },
            WsChannel::Lob(lob_param) => match lob_param {
                Some(LobParam::Bbo { .. }) => {
                    self.ws_loop(
                        TaskEvent::Lob,
                        ws_stream,
                        BinanceWsData::<WsBookTickerBinanceUM>::decode_single,
                    )
                    .await;
                },
                Some(LobParam::Snapshot { .. }) => {
                    self.ws_loop(
                        TaskEvent::Lob,
                        ws_stream,
                        BinanceWsData::<WsPartialDepthBinanceUM>::decode_single,
                    )
                    .await;
                },
                None | Some(LobParam::Incremental { .. }) => {
                    self.ws_loop(
                        TaskEvent::Lob,
                        ws_stream,
                        BinanceWsData::<WsDiffDepthBinanceUM>::decode_single,
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
                    &format!("Unknown Binance UM channel: {:?}", c),
                );
            },
        };
    }

    pub(super) async fn ws_channel_binance_spot(&mut self, ws_stream: &mut WsStream) {
        match &self.ws_info.ws_channel {
            WsChannel::AccountOrders => {
                self.ws_loop(
                    TaskEvent::AccOrder,
                    ws_stream,
                    BinanceWsData::<WsAccountOrderEnvelopeBinanceSpot>::decode_single,
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
                    &format!("Unknown Binance Spot channel: {:?}", c),
                );
            },
        };
    }

    pub(super) async fn ws_channel_binance_cm(&mut self, ws_stream: &mut WsStream) {
        match &self.ws_info.ws_channel {
            WsChannel::Lob(lob_param) => match lob_param {
                Some(LobParam::Bbo { .. }) => {
                    self.ws_loop(
                        TaskEvent::Lob,
                        ws_stream,
                        BinanceWsData::<WsBookTickerBinanceCM>::decode_single,
                    )
                    .await;
                },
                Some(LobParam::Snapshot { .. }) => {
                    self.ws_loop(
                        TaskEvent::Lob,
                        ws_stream,
                        BinanceWsData::<WsPartialDepthBinanceCM>::decode_single,
                    )
                    .await;
                },
                None | Some(LobParam::Incremental { .. }) => {
                    self.ws_loop(
                        TaskEvent::Lob,
                        ws_stream,
                        BinanceWsData::<WsDiffDepthBinanceCM>::decode_single,
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
                    &format!("Unknown Binance CM channel: {:?}", c),
                );
            },
        };
    }
}
