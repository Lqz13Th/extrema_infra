use std::future::ready;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::{from_str, json, Value};
use tracing::info;
use crate::errors::{InfraError, InfraResult};
use crate::market_assets::account_data::{BalanceData, OrderData};
use crate::market_assets::api_general::RequestMethod;
use crate::market_assets::cex::binance::{
    api_key::{read_binance_env_key, BinanceKey},
    api_utils::*,
};
use crate::market_assets::cex::binance::um_futures_rest::exchange_info::RestExchangeInfoBinanceUM;
use crate::market_assets::cex::binance::config_assets::*;
use crate::market_assets::price_data::{CandleData, OrderBookData, TickerData};
use crate::traits::market_cex::{CexPrivateRest, CexPublicRest, MarketCexApi};
use crate::market_assets::base_data::*;
use crate::task_execution::ws_register::*;
use crate::traits::conversion::WsSubscribe;

#[derive(Debug, Clone)]
pub struct BinanceUM {
    pub client: Client,
    pub api_key: Option<BinanceKey>,
}

impl BinanceUM {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            api_key: None
        }
    }
}

impl MarketCexApi for BinanceUM {}


impl CexPublicRest for BinanceUM {
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

    async fn get_live_symbols(&self) -> InfraResult<Vec<String>>{
        self.get_live_symbols().await
    }
}

impl CexPrivateRest for BinanceUM {
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

impl WsSubscribe for BinanceUM {
    async fn ws_cex_pub_subscription(
        &self,
        ws_channel: &WsChannel,
        symbols: &[String]
    ) -> InfraResult<WsSubscription> {
        self.ws_cex_pub_subscription(ws_channel, symbols)
    }

    async fn ws_cex_pri_subscription(
        &self,
        ws_channel: &WsChannel,
    ) -> InfraResult<WsSubscription> {
        self.ws_cex_pri_subscription(ws_channel).await
    }
}

impl BinanceUM {
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
            UM_FUTURES_BASE_URL,
            UM_FUTURES_EXCHANGE_INFO
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

    async fn get_live_symbols(&self) -> InfraResult<Vec<String>> {
        let url = [UM_FUTURES_BASE_URL, UM_FUTURES_EXCHANGE_INFO].concat();

        let response = self.client
            .get(url)
            .send()
            .await?;

        let response_text = response.text().await?;
        let res: RestExchangeInfoBinanceUM = from_str(&response_text)?;

        let perp_symbols: Vec<String> = res.symbols
            .into_iter()
            .filter(|ins| ins.contractType == PERPETUAL && ins.status == TRADING)
            .map(|s| binance_um_to_perp_symbol(&s.symbol))
            .collect();

        Ok(perp_symbols)
    }

    pub async fn create_listen_key(&self) -> InfraResult<BinanceListenKey> {
        let api_key = self.api_key.as_ref().ok_or(InfraError::ApiNotInitialized)?;

        let listen_key: BinanceListenKey = api_key.send_request(
            &self.client,
            RequestMethod::Post,
            None,
            UM_FUTURES_BASE_URL,
            UM_FUTURES_EXCHANGE_INFO
        ).await?;

        Ok(listen_key)
    }

    pub async fn renew_listen_key(&self) -> InfraResult<BinanceListenKey> {
        let api_key = self.api_key.as_ref().ok_or(InfraError::ApiNotInitialized)?;

        let listen_key: BinanceListenKey = api_key.send_request(
            &self.client,
            RequestMethod::Put,
            None,
            UM_FUTURES_BASE_URL,
            UM_FUTURES_EXCHANGE_INFO
        ).await?;

        Ok(listen_key)
    }

    fn ws_cex_pub_subscription(
        &self,
        ws_channel: &WsChannel,
        symbols: &[String]
    ) -> InfraResult<WsSubscription> {
        match ws_channel {
            WsChannel::Account => {
                todo!()
            },
            WsChannel::Candle(channel) => {
                self.ws_candle_subscription(channel, symbols)
            },
            WsChannel::Trades(_) => {
                todo!()
            },
            WsChannel::Tick => {
                todo!()
            },
            WsChannel::Lob => {
                todo!()
            },
            WsChannel::Other(_) => {
                todo!()
            },
        }
    }

    async fn ws_cex_pri_subscription(
        &self,
        ws_channel: &WsChannel
    ) -> InfraResult<WsSubscription> {
        match ws_channel {
            WsChannel::Account => {
                self.ws_account_subscription().await
            },
            _ => {
                Ok(WsSubscription {
                    msg: None,
                    url: UM_FUTURES_WS.to_string(),
                })
            },
        }
    }

    async fn ws_account_subscription(
        &self,
    ) -> InfraResult<WsSubscription> {
        info!("{:?}", self.create_listen_key().await?);
        match self.create_listen_key().await {
            Ok(listen_key) => {
                Ok(WsSubscription {
                    msg: None,
                    url: format!("{}/{}", UM_FUTURES_WS, listen_key.listenKey),
                })
            },
            Err(e) => Err(e)
        }
    }

    fn ws_candle_subscription(
        &self,
        candle_param: &Option<CandleParam>,
        symbols: &[String],
    ) -> InfraResult<WsSubscription> {
        let channel = match candle_param {
            Some(CandleParam::OneSecond) => FUTURE_CANDLE_SUBSCRIPTIONS[0],
            Some(CandleParam::OneMinute) => FUTURE_CANDLE_SUBSCRIPTIONS[1],
            Some(CandleParam::FiveMinutes) => FUTURE_CANDLE_SUBSCRIPTIONS[2],
            Some(CandleParam::FifteenMinutes) => FUTURE_CANDLE_SUBSCRIPTIONS[3],
            Some(CandleParam::OneHour) => FUTURE_CANDLE_SUBSCRIPTIONS[4],
            Some(CandleParam::FourHours) => FUTURE_CANDLE_SUBSCRIPTIONS[5],
            Some(CandleParam::OneDay) => FUTURE_CANDLE_SUBSCRIPTIONS[6],
            Some(CandleParam::OneWeek) => FUTURE_CANDLE_SUBSCRIPTIONS[7],
            None => FUTURE_CANDLE_SUBSCRIPTIONS[1],
        };

        let msg = self.generate_ws_subscription_msg(channel, symbols);

        Ok(WsSubscription {
            msg: Some(msg),
            url: UM_FUTURES_WS.to_string(),
        })
    }

    fn generate_ws_subscription_msg(
        &self,
        param: &str,
        symbols: &[String],
    ) -> String {
        let params: Vec<_> = symbols
            .iter()
            .map(|symbol| {
                format!("{}_perpetual@{}", perp_to_lowercase(symbol), param)
            })
            .collect();

        let subscribe_msg = json!({
            "method": SUBSCRIBE,
            "params": params,
            "id": 1
        });

        subscribe_msg.to_string()
    }
}
