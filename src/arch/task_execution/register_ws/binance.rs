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

use super::{WsStream, WsTaskBuilder};

impl WsTaskBuilder {
    pub(super) async fn ws_channel_binance_um(&mut self, ws_stream: &mut WsStream) {
        match &self.ws_info.ws_channel {
            WsChannel::AccountOrders => {
                self.ws_loop::<BinanceWsData<WsAccountOrderBinanceUM>>(
                    TaskEvent::AccOrder,
                    ws_stream,
                )
                .await;
            },
            WsChannel::AccountBalAndPos => {
                self.ws_loop::<BinanceWsData<WsBalAndPosBinanceUM>>(
                    TaskEvent::AccBalPos,
                    ws_stream,
                )
                .await;
            },
            WsChannel::AccountPositions => {
                self.ws_loop::<BinanceWsData<WsAccountPositionBinanceUM>>(
                    TaskEvent::AccPos,
                    ws_stream,
                )
                .await;
            },
            WsChannel::Candles(..) => {
                self.ws_loop::<BinanceWsData<WsCandleBinanceUM>>(TaskEvent::Candle, ws_stream)
                    .await;
            },
            WsChannel::Trades(..) => {
                self.ws_loop::<BinanceWsData<WsAggTradeBinanceUM>>(TaskEvent::Trade, ws_stream)
                    .await;
            },
            WsChannel::Lob(lob_param) => match lob_param {
                Some(LobParam::Bbo { .. }) => {
                    self.ws_loop::<BinanceWsData<WsBookTickerBinanceUM>>(TaskEvent::Lob, ws_stream)
                        .await;
                },
                Some(LobParam::Snapshot { .. }) => {
                    self.ws_loop::<BinanceWsData<WsPartialDepthBinanceUM>>(
                        TaskEvent::Lob,
                        ws_stream,
                    )
                    .await;
                },
                None | Some(LobParam::Incremental { .. }) => {
                    self.ws_loop::<BinanceWsData<WsDiffDepthBinanceUM>>(TaskEvent::Lob, ws_stream)
                        .await;
                },
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
                self.ws_loop::<BinanceWsData<WsAccountOrderEnvelopeBinanceSpot>>(
                    TaskEvent::AccOrder,
                    ws_stream,
                )
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
                    self.ws_loop::<BinanceWsData<WsBookTickerBinanceCM>>(TaskEvent::Lob, ws_stream)
                        .await;
                },
                Some(LobParam::Snapshot { .. }) => {
                    self.ws_loop::<BinanceWsData<WsPartialDepthBinanceCM>>(
                        TaskEvent::Lob,
                        ws_stream,
                    )
                    .await;
                },
                None | Some(LobParam::Incremental { .. }) => {
                    self.ws_loop::<BinanceWsData<WsDiffDepthBinanceCM>>(TaskEvent::Lob, ws_stream)
                        .await;
                },
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
