use std::future::{ready, Future};


use crate::errors::{InfraError, InfraResult};
use crate::market_assets::{
    account_data::*,
    price_data::*,
    utils_data::*,
};

pub trait MarketCexApi: CexPublicRest + CexPrivateRest {}

pub trait CexPublicRest: Send + Sync {
    fn get_ticker(
        &self,
        _symbols: Vec<String>,
    ) -> impl Future<Output = InfraResult<Vec<TickerData>>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    fn get_orderbook(
        &self,
        _symbols: Vec<String>,
        _depth: usize
    ) -> impl Future<Output = InfraResult<Vec<OrderBookData>>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    fn get_candles(
        &self,
        _symbols: Vec<String>,
        _interval: &str
    ) -> impl Future<Output = InfraResult<Vec<CandleData>>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    fn get_market_info(
        &self,
        _symbols: Vec<String>,
    ) -> impl Future<Output = InfraResult<Vec<MarketInfoData>>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    fn get_live_symbols(&self) -> impl Future<Output = InfraResult<Vec<String>>> + Send {
        ready(Err(InfraError::Unimplemented))
    }
}

pub trait CexPrivateRest: Send + Sync {
    fn init_api_key(&mut self);

    fn place_order(
        &self,
        _symbol: String,
        _side: String,
        _price: Option<f64>,
        _quantity: f64,
    ) -> impl Future<Output = InfraResult<OrderData>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    fn cancel_orders(
        &self,
        _symbols: Vec<String>,
    ) -> impl Future<Output = InfraResult<OrderData>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    fn get_balance(
        &self,
        _assets: Vec<String>,
    ) -> impl Future<Output = InfraResult<Vec<BalanceData>>> + Send {
        ready(Err(InfraError::Unimplemented))
    }

    fn get_position(
        &self,
        _symbols: Vec<String>,
    ) -> impl Future<Output = InfraResult<Vec<PositionData>>> + Send {
        ready(Err(InfraError::Unimplemented))
    }
}
