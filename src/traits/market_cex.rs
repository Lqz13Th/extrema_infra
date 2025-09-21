use std::future::{ready, Future};

use crate::errors::{InfraError, InfraResult};
use crate::market_assets::{
    base_data::InstrumentType,
    account_data::*,
    price_data::*,
    utils_data::*,
};
use crate::task_execution::task_ws::WsChannel;

pub trait CexWebsocket: Send + Sync {
    fn get_public_sub_msg(
        &self,
        _channel: &WsChannel,
        _insts: Option<&[String]>,
    ) -> impl Future<Output = InfraResult<String>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    fn get_private_sub_msg(
        &self,
        _channel: &WsChannel,
    ) -> impl Future<Output = InfraResult<String>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    fn get_public_connect_msg(
        &self,
        _channel: &WsChannel,
    ) -> impl Future<Output = InfraResult<String>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    fn get_private_connect_msg(
        &self,
        _channel: &WsChannel,
    ) -> impl Future<Output = InfraResult<String>> + Send {
        ready(Err(InfraError::Unimplemented))
    }
}

pub trait MarketCexApi: CexPublicRest + CexPrivateRest {}

pub trait CexPublicRest: Send + Sync {
    fn get_ticker(
        &self,
        _insts: Option<&[String]>,
    ) -> impl Future<Output = InfraResult<Vec<TickerData>>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    fn get_orderbook(
        &self,
        _insts: Option<&[String]>,
        _depth: usize,
    ) -> impl Future<Output = InfraResult<Vec<OrderBookData>>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    fn get_candles(
        &self,
        _insts: Option<&[String]>,
        _interval: &str
    ) -> impl Future<Output = InfraResult<Vec<CandleData>>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    fn get_instrument_info(
        &self,
        _inst_type: InstrumentType,
    ) -> impl Future<Output = InfraResult<Vec<InstrumentInfo>>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    fn get_live_instruments(&self) -> impl Future<Output = InfraResult<Vec<String>>> + Send {
        ready(Err(InfraError::Unimplemented))
    }
}

pub trait CexPrivateRest: Send + Sync {
    fn init_api_key(&mut self);

    fn place_order(
        &self,
        _inst: String,
        _side: String,
        _price: Option<f64>,
        _quantity: f64,
    ) -> impl Future<Output = InfraResult<OrderData>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    fn cancel_orders(
        &self,
        _insts: Vec<String>,
    ) -> impl Future<Output = InfraResult<OrderData>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    fn get_balance(
        &self,
        _insts: Option<&[String]>,
    ) -> impl Future<Output = InfraResult<Vec<BalanceData>>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    fn get_position(
        &self,
        _insts: Option<&[String]>,
    ) -> impl Future<Output = InfraResult<Vec<PositionData>>> + Send {
        ready(Err(InfraError::Unimplemented))
    }
}
