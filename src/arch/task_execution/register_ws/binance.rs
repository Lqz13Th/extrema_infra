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
    strategy_base::handler::handler_core::{
        find_acc_bal_pos, find_acc_order, find_acc_pos, find_candle, find_lob, find_trade,
    },
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
                if let Some(tx) = find_acc_order(&self.board_cast_channel) {
                    self.ws_loop::<BinanceWsData<WsAccountOrderBinanceUM>>(tx, ws_stream)
                        .await;
                } else {
                    self.log(
                        LogLevel::Warn,
                        "No broadcast channel found for Binance UmFutures Acc order",
                    );
                }
            },
            WsChannel::AccountBalAndPos => {
                if let Some(tx) = find_acc_bal_pos(&self.board_cast_channel) {
                    self.ws_loop::<BinanceWsData<WsBalAndPosBinanceUM>>(tx, ws_stream)
                        .await;
                } else {
                    self.log(
                        LogLevel::Warn,
                        "No broadcast channel found for Binance UmFutures Acc Bal and Pos",
                    );
                }
            },
            WsChannel::AccountPositions => {
                if let Some(tx) = find_acc_pos(&self.board_cast_channel) {
                    self.ws_loop::<BinanceWsData<WsAccountPositionBinanceUM>>(tx, ws_stream)
                        .await;
                } else {
                    self.log(
                        LogLevel::Warn,
                        "No broadcast channel found for Binance UmFutures Acc Position",
                    );
                }
            },
            WsChannel::Candles(..) => {
                if let Some(tx) = find_candle(&self.board_cast_channel) {
                    self.ws_loop::<BinanceWsData<WsCandleBinanceUM>>(tx, ws_stream)
                        .await;
                } else {
                    self.log(
                        LogLevel::Warn,
                        "No broadcast channel found for Binance UmFutures Candles",
                    );
                }
            },
            WsChannel::Trades(..) => {
                if let Some(tx) = find_trade(&self.board_cast_channel) {
                    self.ws_loop::<BinanceWsData<WsAggTradeBinanceUM>>(tx, ws_stream)
                        .await;
                } else {
                    self.log(
                        LogLevel::Warn,
                        "No broadcast channel found for Binance UmFutures Trades",
                    );
                }
            },
            WsChannel::Lob(lob_param) => {
                if let Some(tx) = find_lob(&self.board_cast_channel) {
                    match lob_param {
                        Some(LobParam::Bbo { .. }) => {
                            self.ws_loop::<BinanceWsData<WsBookTickerBinanceUM>>(tx, ws_stream)
                                .await;
                        },
                        Some(LobParam::Snapshot { .. }) => {
                            self.ws_loop::<BinanceWsData<WsPartialDepthBinanceUM>>(tx, ws_stream)
                                .await;
                        },
                        None | Some(LobParam::Incremental { .. }) => {
                            self.ws_loop::<BinanceWsData<WsDiffDepthBinanceUM>>(tx, ws_stream)
                                .await;
                        },
                    }
                } else {
                    self.log(
                        LogLevel::Warn,
                        "No broadcast channel found for Binance UmFutures LOB",
                    );
                }
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
                if let Some(tx) = find_acc_order(&self.board_cast_channel) {
                    self.ws_loop::<BinanceWsData<WsAccountOrderEnvelopeBinanceSpot>>(tx, ws_stream)
                        .await;
                } else {
                    self.log(
                        LogLevel::Warn,
                        "No broadcast channel found for Binance Spot Acc order",
                    );
                }
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
            WsChannel::Lob(lob_param) => {
                if let Some(tx) = find_lob(&self.board_cast_channel) {
                    match lob_param {
                        Some(LobParam::Bbo { .. }) => {
                            self.ws_loop::<BinanceWsData<WsBookTickerBinanceCM>>(tx, ws_stream)
                                .await;
                        },
                        Some(LobParam::Snapshot { .. }) => {
                            self.ws_loop::<BinanceWsData<WsPartialDepthBinanceCM>>(tx, ws_stream)
                                .await;
                        },
                        None | Some(LobParam::Incremental { .. }) => {
                            self.ws_loop::<BinanceWsData<WsDiffDepthBinanceCM>>(tx, ws_stream)
                                .await;
                        },
                    }
                } else {
                    self.log(
                        LogLevel::Warn,
                        "No broadcast channel found for Binance CmFutures LOB",
                    );
                }
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
