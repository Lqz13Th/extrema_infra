#![allow(unused_imports)]

use super::prelude::*;
use crate::arch::{
    market_assets::{
        api_data::{account_data::*, price_data::*, utils_data::*},
        api_general::OrderParams,
        base_data::InstrumentType,
    },
    task_execution::task_ws::{CandleParam, WsChannel},
    traits::market_cex::*,
};
use crate::errors::{InfraError, InfraResult};

#[derive(Clone, Debug)]
#[cfg(feature = "cex_clients")]
pub enum CexClients {
    BinanceCm(BinanceCmCli),
    BinanceUm(BinanceUmCli),
    Okx(OkxCli),
}

#[cfg(feature = "cex_clients")]
impl Default for CexClients {
    fn default() -> Self {
        CexClients::Okx(OkxCli::default())
    }
}

#[cfg(feature = "cex_clients")]
impl MarketCexApi for CexClients {}

#[cfg(feature = "cex_clients")]
impl CexPublicRest for CexClients {
    async fn get_ticker(&self, inst: &str) -> InfraResult<TickerData> {
        match self {
            CexClients::BinanceCm(c) => c.get_ticker(inst).await,
            CexClients::BinanceUm(c) => c.get_ticker(inst).await,
            CexClients::Okx(c) => c.get_ticker(inst).await,
        }
    }

    async fn get_orderbook(&self, inst: &str, depth: usize) -> InfraResult<OrderBookData> {
        match self {
            CexClients::BinanceCm(c) => c.get_orderbook(inst, depth).await,
            CexClients::BinanceUm(c) => c.get_orderbook(inst, depth).await,
            CexClients::Okx(c) => c.get_orderbook(inst, depth).await,
        }
    }

    async fn get_candles(&self, inst: &str, interval: CandleParam) -> InfraResult<Vec<CandleData>> {
        match self {
            CexClients::BinanceCm(c) => c.get_candles(inst, interval).await,
            CexClients::BinanceUm(c) => c.get_candles(inst, interval).await,
            CexClients::Okx(c) => c.get_candles(inst, interval).await,
        }
    }

    async fn get_instrument_info(
        &self,
        inst_type: InstrumentType,
    ) -> InfraResult<Vec<InstrumentInfo>> {
        match self {
            CexClients::BinanceCm(c) => c.get_instrument_info(inst_type).await,
            CexClients::BinanceUm(c) => c.get_instrument_info(inst_type).await,
            CexClients::Okx(c) => c.get_instrument_info(inst_type).await,
        }
    }

    async fn get_live_instruments(&self) -> InfraResult<Vec<String>> {
        match self {
            CexClients::BinanceCm(c) => c.get_live_instruments().await,
            CexClients::BinanceUm(c) => c.get_live_instruments().await,
            CexClients::Okx(c) => c.get_live_instruments().await,
        }
    }
}

#[cfg(feature = "cex_clients")]
impl CexPrivateRest for CexClients {
    fn init_api_key(&mut self) {
        match self {
            CexClients::BinanceCm(c) => c.init_api_key(),
            CexClients::BinanceUm(c) => c.init_api_key(),
            CexClients::Okx(c) => c.init_api_key(),
        }
    }

    async fn place_order(&self, order_params: OrderParams) -> InfraResult<OrderAckData> {
        match self {
            CexClients::BinanceCm(c) => c.place_order(order_params).await,
            CexClients::BinanceUm(c) => c.place_order(order_params).await,
            CexClients::Okx(c) => c.place_order(order_params).await,
        }
    }

    async fn cancel_order(
        &self,
        inst: &str,
        order_id: Option<&str>,
        cli_order_id: Option<&str>,
    ) -> InfraResult<OrderAckData> {
        match self {
            CexClients::BinanceCm(c) => c.cancel_order(inst, order_id, cli_order_id).await,
            CexClients::BinanceUm(c) => c.cancel_order(inst, order_id, cli_order_id).await,
            CexClients::Okx(c) => c.cancel_order(inst, order_id, cli_order_id).await,
        }
    }

    async fn get_balance(&self, insts: Option<&[String]>) -> InfraResult<Vec<BalanceData>> {
        match self {
            CexClients::BinanceCm(c) => c.get_balance(insts).await,
            CexClients::BinanceUm(c) => c.get_balance(insts).await,
            CexClients::Okx(c) => c.get_balance(insts).await,
        }
    }

    async fn get_positions(&self, insts: Option<&[String]>) -> InfraResult<Vec<PositionData>> {
        match self {
            CexClients::BinanceCm(c) => c.get_positions(insts).await,
            CexClients::BinanceUm(c) => c.get_positions(insts).await,
            CexClients::Okx(c) => c.get_positions(insts).await,
        }
    }

    async fn get_order_history(
        &self,
        inst: &str,
        start_time: Option<u64>,
        end_time: Option<u64>,
        limit: Option<u32>,
        order_id: Option<u64>,
    ) -> InfraResult<Vec<HistoricalOrder>> {
        match self {
            CexClients::BinanceUm(c) => {
                c.get_order_history(inst, start_time, end_time, limit, order_id)
                    .await
            },
            _ => Err(InfraError::Unimplemented),
        }
    }
}

#[cfg(feature = "cex_clients")]
impl CexWebsocket for CexClients {
    async fn get_public_sub_msg(
        &self,
        channel: &WsChannel,
        insts: Option<&[String]>,
    ) -> InfraResult<String> {
        match self {
            CexClients::BinanceCm(c) => c.get_public_sub_msg(channel, insts).await,
            CexClients::BinanceUm(c) => c.get_public_sub_msg(channel, insts).await,
            CexClients::Okx(c) => c.get_public_sub_msg(channel, insts).await,
        }
    }

    async fn get_private_sub_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        match self {
            CexClients::BinanceCm(c) => c.get_private_sub_msg(channel).await,
            CexClients::BinanceUm(c) => c.get_private_sub_msg(channel).await,
            CexClients::Okx(c) => c.get_private_sub_msg(channel).await,
        }
    }

    async fn get_public_connect_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        match self {
            CexClients::BinanceCm(c) => c.get_public_connect_msg(channel).await,
            CexClients::BinanceUm(c) => c.get_public_connect_msg(channel).await,
            CexClients::Okx(c) => c.get_public_connect_msg(channel).await,
        }
    }

    async fn get_private_connect_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        match self {
            CexClients::BinanceCm(c) => c.get_private_connect_msg(channel).await,
            CexClients::BinanceUm(c) => c.get_private_connect_msg(channel).await,
            CexClients::Okx(c) => c.get_private_connect_msg(channel).await,
        }
    }
}
