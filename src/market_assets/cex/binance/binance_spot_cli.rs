use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::{from_str, Value};

use crate::errors::*;
use crate::traits::market_cex::*;
use crate::market_assets::{
    cex::binance::{
        config_assets::*,
        api_key::*
    },
    account_data::*,
    market_data::*,
    rules_data::*,
    api_genral::*
};

#[derive(Debug, Clone)]
pub struct BinanceSpot {
    pub client: Client,
    pub api_key: Option<BinanceKey>,
}

impl BinanceSpot {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            api_key: None
        }
    }
}

impl CexPublicRest for BinanceSpot {
    async fn get_ticker(
        &self,
        symbols: Vec<String>,
    ) -> InfraResult<Vec<TickerData>> {
        self.get_spot_ticker(symbols).await
    }

    async fn get_orderbook(
        &self,
        symbols: Vec<String>,
        depth: usize,
    ) -> InfraResult<Vec<OrderBookData>> {
        self.get_spot_orderbook(symbols, depth).await
    }

    async fn get_candles(
        &self,
        symbols: Vec<String>,
        interval: &str,
    ) -> InfraResult<Vec<CandleData>> {
        self.get_spot_candles(symbols, interval).await
    }
}

impl CexPrivateRest for BinanceSpot {
    async fn place_order(
        &self,
        symbol: String,
        side: String,
        price: Option<f64>,
        quantity: f64,
    ) -> InfraResult<OrderData> {
        self.place_spot_order(symbol, side, price, quantity).await
    }

    async fn get_balance(
        &self,
        assets: Vec<String>,
    ) -> InfraResult<Vec<BalanceData>> {
        self.get_spot_balance(assets).await
    }
}

impl MarketCexApi for BinanceSpot {}

impl BinanceSpot {
    async fn get_spot_ticker(&self, symbols: Vec<String>) -> InfraResult<Vec<TickerData>> {
        // TODO: 调用 SPOT ticker endpoint
        todo!()
    }

    async fn get_spot_orderbook(
        &self,
        symbols: Vec<String>,
        depth: usize
    ) -> InfraResult<Vec<OrderBookData>> {
        // TODO: 调用 SPOT orderbook endpoint
        todo!()
    }

    async fn get_spot_candles(
        &self,
        symbols: Vec<String>,
        interval: &str
    ) -> InfraResult<Vec<CandleData>> {
        // TODO: 调用 SPOT Kline endpoint
        todo!()
    }

    async fn get_spot_balance(
        &self,
        assets: Vec<String>,
    ) -> InfraResult<Vec<BalanceData>> {
        let api_key = self.api_key.as_ref().ok_or(InfraError::ApiNotInitialized)?;

        let all_balances: Vec<BalanceData> = api_key.send_request(
            &self.client,
            RequestMethod::Get,
            None,
            SPOT_BASE_URL,
            SPOT_EXCHANGE_INFO
        ).await?;

        let filtered = if assets.is_empty() {
            all_balances
        } else {
            all_balances
                .into_iter()
                .filter(|b| assets.contains(&b.asset))
                .collect()
        };

        Ok(filtered)    
    }

    async fn place_spot_order(
        &self,
        symbol: String,
        side: String,
        price: Option<f64>,
        quantity: f64,
    ) -> InfraResult<OrderData> {
        // TODO: 调用 SPOT 下单 endpoint
        todo!()
    }
}
