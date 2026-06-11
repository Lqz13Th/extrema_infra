use std::future::{Future, ready};

use crate::arch::{
    market_assets::{
        api_data::{account_data::*, price_data::*, utils_data::*},
        api_general::OrderParams,
        base_data::InstrumentType,
    },
    task_execution::task_ws::{CandleParam, WsChannel},
};
use crate::errors::{InfraError, InfraResult};

/// Marker trait for a limit-order-book market client with public and private
/// REST support.
///
/// Implement this for exchange clients that satisfy both [`LobPublicRest`] and
/// [`LobPrivateRest`].
pub trait MarketLobApi: LobPublicRest + LobPrivateRest {}

/// Public REST operations for LOB-style exchanges.
///
/// Methods default to [`InfraError::Unimplemented`], so an exchange client can
/// implement only the endpoints it supports.
pub trait LobPublicRest: Send + Sync {
    /// Fetches tickers, optionally restricted by instruments and instrument type.
    fn get_tickers(
        &self,
        _insts: Option<&[String]>,
        _inst_type: Option<InstrumentType>,
    ) -> impl Future<Output = InfraResult<Vec<TickerData>>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    /// Fetches one order book snapshot.
    fn get_orderbook(
        &self,
        _inst: &str,
        _depth: usize,
    ) -> impl Future<Output = InfraResult<OrderBookData>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    /// Fetches historical candles for one instrument.
    fn get_candles(
        &self,
        _inst: &str,
        _interval: CandleParam,
        _limit: Option<u32>,
        _start_time_us: Option<u64>,
        _end_time_us: Option<u64>,
    ) -> impl Future<Output = InfraResult<Vec<CandleData>>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    /// Fetches exchange instrument metadata.
    fn get_instrument_info(
        &self,
        _inst_type: InstrumentType,
    ) -> impl Future<Output = InfraResult<Vec<InstrumentInfo>>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    /// Fetches currently live instruments.
    fn get_live_instruments(
        &self,
        _inst_type: InstrumentType,
    ) -> impl Future<Output = InfraResult<Vec<String>>> + Send {
        ready(Err(InfraError::Unimplemented))
    }
}

/// Private REST operations for LOB-style exchanges.
///
/// Exchange clients should initialize credentials with [`init_api_key`] before
/// private calls are made.
///
/// [`init_api_key`]: LobPrivateRest::init_api_key
pub trait LobPrivateRest: Send + Sync {
    /// Loads API credentials into the exchange client.
    fn init_api_key(&mut self);

    /// Places one order.
    fn place_order(
        &self,
        _order_params: OrderParams,
    ) -> impl Future<Output = InfraResult<OrderAckData>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    /// Cancels one order by exchange order id or client order id.
    fn cancel_order(
        &self,
        _inst: &str,
        _order_id: Option<&str>,
        _cli_order_id: Option<&str>,
    ) -> impl Future<Output = InfraResult<OrderAckData>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    /// Fetches account balances.
    fn get_balance(
        &self,
        _insts: Option<&[String]>,
    ) -> impl Future<Output = InfraResult<Vec<BalanceData>>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    /// Fetches open positions.
    fn get_positions(
        &self,
        _insts: Option<&[String]>,
    ) -> impl Future<Output = InfraResult<Vec<PositionData>>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    /// Fetches historical orders.
    fn get_order_history(
        &self,
        _inst: &str,
        _start_time: Option<u64>,
        _end_time: Option<u64>,
        _limit: Option<u32>,
        _order_id: Option<&str>,
    ) -> impl Future<Output = InfraResult<Vec<HistoOrderData>>> + Send {
        ready(Err(InfraError::Unimplemented))
    }
}

/// Websocket message builder for LOB-style exchanges.
///
/// Implementations return exchange-specific connect and subscription payloads.
/// Authentication or login payloads are exchange-specific helper APIs on the
/// concrete client. The websocket relay task owns IO; strategies send these
/// payloads to the relay through command handles.
pub trait LobWebsocket: Send + Sync {
    /// Builds a public subscription message.
    fn get_public_sub_msg(
        &self,
        _channel: &WsChannel,
        _insts: Option<&[String]>,
    ) -> impl Future<Output = InfraResult<String>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    /// Builds a private subscription message.
    fn get_private_sub_msg(
        &self,
        _channel: &WsChannel,
    ) -> impl Future<Output = InfraResult<String>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    /// Builds or returns a public websocket connection target.
    fn get_public_connect_msg(
        &self,
        _channel: &WsChannel,
    ) -> impl Future<Output = InfraResult<String>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    /// Builds or returns a private websocket connection target.
    fn get_private_connect_msg(
        &self,
        _channel: &WsChannel,
    ) -> impl Future<Output = InfraResult<String>> + Send {
        ready(Err(InfraError::Unimplemented))
    }
}
