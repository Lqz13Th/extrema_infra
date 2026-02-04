#![allow(unused_imports)]

use super::prelude::*;
use crate::arch::{
    market_assets::{
        api_data::{account_data::*, price_data::*, utils_data::*},
        api_general::OrderParams,
        base_data::InstrumentType,
    },
    task_execution::task_ws::{CandleParam, WsChannel},
    traits::market_lob::*,
};
use crate::errors::{InfraError, InfraResult};

#[derive(Clone, Debug)]
#[cfg(feature = "lob_clients")]
pub enum LobClients {
    Hyperliquid(HyperliquidCli),
    BinanceCm(BinanceCmCli),
    BinanceUm(BinanceUmCli),
    GateDelivery(GateDeliveryCli),
    GateFutures(GateFuturesCli),
    GateSpot(GateSpotCli),
    Okx(OkxCli),
}

#[cfg(feature = "lob_clients")]
impl Default for LobClients {
    fn default() -> Self {
        LobClients::Hyperliquid(HyperliquidCli::default())
    }
}

#[cfg(feature = "lob_clients")]
impl MarketLobApi for LobClients {}

#[cfg(feature = "lob_clients")]
impl LobPublicRest for LobClients {
    async fn get_ticker(&self, inst: &str) -> InfraResult<TickerData> {
        match self {
            LobClients::Hyperliquid(c) => c.get_ticker(inst).await,
            LobClients::BinanceCm(c) => c.get_ticker(inst).await,
            LobClients::BinanceUm(c) => c.get_ticker(inst).await,
            LobClients::GateDelivery(c) => c.get_ticker(inst).await,
            LobClients::GateFutures(c) => c.get_ticker(inst).await,
            LobClients::GateSpot(c) => c.get_ticker(inst).await,
            LobClients::Okx(c) => c.get_ticker(inst).await,
        }
    }

    async fn get_orderbook(&self, inst: &str, depth: usize) -> InfraResult<OrderBookData> {
        match self {
            LobClients::Hyperliquid(c) => c.get_orderbook(inst, depth).await,
            LobClients::BinanceCm(c) => c.get_orderbook(inst, depth).await,
            LobClients::BinanceUm(c) => c.get_orderbook(inst, depth).await,
            LobClients::GateDelivery(c) => c.get_orderbook(inst, depth).await,
            LobClients::GateFutures(c) => c.get_orderbook(inst, depth).await,
            LobClients::GateSpot(c) => c.get_orderbook(inst, depth).await,
            LobClients::Okx(c) => c.get_orderbook(inst, depth).await,
        }
    }

    async fn get_candles(&self, inst: &str, interval: CandleParam) -> InfraResult<Vec<CandleData>> {
        match self {
            LobClients::Hyperliquid(c) => c.get_candles(inst, interval).await,
            LobClients::BinanceCm(c) => c.get_candles(inst, interval).await,
            LobClients::BinanceUm(c) => c.get_candles(inst, interval).await,
            LobClients::GateDelivery(c) => c.get_candles(inst, interval).await,
            LobClients::GateFutures(c) => c.get_candles(inst, interval).await,
            LobClients::GateSpot(c) => c.get_candles(inst, interval).await,
            LobClients::Okx(c) => c.get_candles(inst, interval).await,
        }
    }

    async fn get_instrument_info(
        &self,
        inst_type: InstrumentType,
    ) -> InfraResult<Vec<InstrumentInfo>> {
        match self {
            LobClients::Hyperliquid(c) => c.get_instrument_info(inst_type).await,
            LobClients::BinanceCm(c) => c.get_instrument_info(inst_type).await,
            LobClients::BinanceUm(c) => c.get_instrument_info(inst_type).await,
            LobClients::GateDelivery(c) => c.get_instrument_info(inst_type).await,
            LobClients::GateFutures(c) => c.get_instrument_info(inst_type).await,
            LobClients::GateSpot(c) => c.get_instrument_info(inst_type).await,
            LobClients::Okx(c) => c.get_instrument_info(inst_type).await,
        }
    }
}

#[cfg(feature = "lob_clients")]
impl LobPrivateRest for LobClients {
    fn init_api_key(&mut self) {
        match self {
            LobClients::Hyperliquid(c) => c.init_api_key(),
            LobClients::BinanceCm(c) => c.init_api_key(),
            LobClients::BinanceUm(c) => c.init_api_key(),
            LobClients::GateDelivery(c) => c.init_api_key(),
            LobClients::GateFutures(c) => c.init_api_key(),
            LobClients::GateSpot(c) => c.init_api_key(),
            LobClients::Okx(c) => c.init_api_key(),
        }
    }

    async fn place_order(&self, order_params: OrderParams) -> InfraResult<OrderAckData> {
        match self {
            LobClients::Hyperliquid(c) => c.place_order(order_params).await,
            LobClients::BinanceCm(c) => c.place_order(order_params).await,
            LobClients::BinanceUm(c) => c.place_order(order_params).await,
            LobClients::GateDelivery(c) => c.place_order(order_params).await,
            LobClients::GateFutures(c) => c.place_order(order_params).await,
            LobClients::GateSpot(c) => c.place_order(order_params).await,
            LobClients::Okx(c) => c.place_order(order_params).await,
        }
    }

    async fn cancel_order(
        &self,
        inst: &str,
        order_id: Option<&str>,
        cli_order_id: Option<&str>,
    ) -> InfraResult<OrderAckData> {
        match self {
            LobClients::Hyperliquid(c) => c.cancel_order(inst, order_id, cli_order_id).await,
            LobClients::BinanceCm(c) => c.cancel_order(inst, order_id, cli_order_id).await,
            LobClients::BinanceUm(c) => c.cancel_order(inst, order_id, cli_order_id).await,
            LobClients::GateDelivery(c) => c.cancel_order(inst, order_id, cli_order_id).await,
            LobClients::GateFutures(c) => c.cancel_order(inst, order_id, cli_order_id).await,
            LobClients::GateSpot(c) => c.cancel_order(inst, order_id, cli_order_id).await,
            LobClients::Okx(c) => c.cancel_order(inst, order_id, cli_order_id).await,
        }
    }

    async fn get_balance(&self, insts: Option<&[String]>) -> InfraResult<Vec<BalanceData>> {
        match self {
            LobClients::Hyperliquid(c) => c.get_balance(insts).await,
            LobClients::BinanceCm(c) => c.get_balance(insts).await,
            LobClients::BinanceUm(c) => c.get_balance(insts).await,
            LobClients::GateDelivery(c) => c.get_balance(insts).await,
            LobClients::GateFutures(c) => c.get_balance(insts).await,
            LobClients::GateSpot(c) => c.get_balance(insts).await,
            LobClients::Okx(c) => c.get_balance(insts).await,
        }
    }

    async fn get_positions(&self, insts: Option<&[String]>) -> InfraResult<Vec<PositionData>> {
        match self {
            LobClients::Hyperliquid(c) => c.get_positions(insts).await,
            LobClients::BinanceCm(c) => c.get_positions(insts).await,
            LobClients::BinanceUm(c) => c.get_positions(insts).await,
            LobClients::GateDelivery(c) => c.get_positions(insts).await,
            LobClients::GateFutures(c) => c.get_positions(insts).await,
            LobClients::GateSpot(c) => c.get_positions(insts).await,
            LobClients::Okx(c) => c.get_positions(insts).await,
        }
    }

    async fn get_order_history(
        &self,
        inst: &str,
        start_time: Option<u64>,
        end_time: Option<u64>,
        limit: Option<u32>,
        order_id: Option<u64>,
    ) -> InfraResult<Vec<HistoOrderData>> {
        match self {
            LobClients::BinanceUm(c) => {
                c.get_order_history(inst, start_time, end_time, limit, order_id)
                    .await
            },
            _ => Err(InfraError::Unimplemented),
        }
    }
}

#[cfg(feature = "lob_clients")]
impl LobWebsocket for LobClients {
    async fn get_public_sub_msg(
        &self,
        channel: &WsChannel,
        insts: Option<&[String]>,
    ) -> InfraResult<String> {
        match self {
            LobClients::Hyperliquid(c) => c.get_public_sub_msg(channel, insts).await,
            LobClients::BinanceCm(c) => c.get_public_sub_msg(channel, insts).await,
            LobClients::BinanceUm(c) => c.get_public_sub_msg(channel, insts).await,
            LobClients::GateDelivery(c) => c.get_public_sub_msg(channel, insts).await,
            LobClients::GateFutures(c) => c.get_public_sub_msg(channel, insts).await,
            LobClients::GateSpot(c) => c.get_public_sub_msg(channel, insts).await,
            LobClients::Okx(c) => c.get_public_sub_msg(channel, insts).await,
        }
    }

    async fn get_private_sub_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        match self {
            LobClients::Hyperliquid(c) => c.get_private_sub_msg(channel).await,
            LobClients::BinanceCm(c) => c.get_private_sub_msg(channel).await,
            LobClients::BinanceUm(c) => c.get_private_sub_msg(channel).await,
            LobClients::GateDelivery(c) => c.get_private_sub_msg(channel).await,
            LobClients::GateFutures(c) => c.get_private_sub_msg(channel).await,
            LobClients::GateSpot(c) => c.get_private_sub_msg(channel).await,
            LobClients::Okx(c) => c.get_private_sub_msg(channel).await,
        }
    }

    async fn get_public_connect_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        match self {
            LobClients::Hyperliquid(c) => c.get_public_connect_msg(channel).await,
            LobClients::BinanceCm(c) => c.get_public_connect_msg(channel).await,
            LobClients::BinanceUm(c) => c.get_public_connect_msg(channel).await,
            LobClients::GateDelivery(c) => c.get_public_connect_msg(channel).await,
            LobClients::GateFutures(c) => c.get_public_connect_msg(channel).await,
            LobClients::GateSpot(c) => c.get_public_connect_msg(channel).await,
            LobClients::Okx(c) => c.get_public_connect_msg(channel).await,
        }
    }

    async fn get_private_connect_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        match self {
            LobClients::Hyperliquid(c) => c.get_private_connect_msg(channel).await,
            LobClients::BinanceCm(c) => c.get_private_connect_msg(channel).await,
            LobClients::BinanceUm(c) => c.get_private_connect_msg(channel).await,
            LobClients::GateDelivery(c) => c.get_private_connect_msg(channel).await,
            LobClients::GateFutures(c) => c.get_private_connect_msg(channel).await,
            LobClients::GateSpot(c) => c.get_private_connect_msg(channel).await,
            LobClients::Okx(c) => c.get_private_connect_msg(channel).await,
        }
    }
}
