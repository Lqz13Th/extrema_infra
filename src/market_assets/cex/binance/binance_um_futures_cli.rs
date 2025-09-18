use std::{
    sync::Arc,
    collections::HashMap,
};
use serde_json::{from_str, json};
use reqwest::Client;

use tracing::{error, info};

use crate::errors::{InfraError, InfraResult};

use crate::market_assets::{
    api_general::RequestMethod,
    base_data::*,
    account_data::*,
};
use crate::task_execution::task_ws::*;

use crate::traits::{
    conversion::WsSubscribe,
    market_cex::{CexPrivateRest, CexPublicRest, MarketCexApi}
};

use super::{
    api_key::{BinanceKey, read_binance_env_key},
    api_utils::*,
    config_assets::*,
    um_futures_rest::exchange_info::RestExchangeInfoBinanceUM,
};

fn create_binance_cli_with_key(
    keys: HashMap<String, BinanceKey>,
    shared_client: Arc<Client>,
) -> HashMap<String, BinanceUmCli> {
    keys.into_iter()
        .map(|(id, key)| {
            let cli = BinanceUmCli {
                client: shared_client.clone(),
                api_key: Some(key),
            };
            (id, cli)
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct BinanceUmCli {
    pub client: Arc<Client>,
    pub api_key: Option<BinanceKey>,
}

impl MarketCexApi for BinanceUmCli {}


impl CexPublicRest for BinanceUmCli {
    async fn get_live_symbols(&self) -> InfraResult<Vec<String>>{
        self.get_live_symbols().await
    }
}

impl CexPrivateRest for BinanceUmCli {
    fn init_api_key(&mut self) {
        match read_binance_env_key() {
            Ok(binance_key) => {
                self.api_key = Some(binance_key);
            },
            Err(e) => {
                error!("Failed to read BINANCE env key: {:?}", e);
            }
        }
    }

    async fn get_balance(
        &self,
        assets: Vec<String>,
    ) -> InfraResult<Vec<BalanceData>> {
        self.get_balance(assets).await
    }
}

impl WsSubscribe for BinanceUmCli {
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

impl Default for BinanceUmCli {
    fn default() -> Self {
        Self::new(Arc::new(Client::new()))
    }
}

impl BinanceUmCli {
    pub fn new(shared_client: Arc<Client>) -> Self {
        Self {
            client: shared_client,
            api_key: None
        }
    }

    async fn get_balance(
        &self,
        assets: Vec<String>,
    ) -> InfraResult<Vec<BalanceData>> {
        let api_key = self.api_key.as_ref().ok_or(InfraError::ApiNotInitialized)?;

        let all_balances: Vec<BalanceData> = api_key.send_signed_request(
            &self.client,
            RequestMethod::Get,
            None,
            BINANCE_UM_FUTURES_BASE_URL,
            BINANCE_UM_FUTURES_EXCHANGE_INFO
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

    async fn get_live_symbols(&self) -> InfraResult<Vec<String>> {
        let url = [BINANCE_UM_FUTURES_BASE_URL, BINANCE_UM_FUTURES_EXCHANGE_INFO].concat();

        let response = self.client
            .get(url)
            .send()
            .await?;

        let response_text = response.text().await?;
        let res: RestExchangeInfoBinanceUM = from_str(&response_text)?;

        let perp_symbols: Vec<String> = res.symbols
            .into_iter()
            .filter(|ins| ins.contractType == PERPETUAL && ins.status == TRADING)
            .map(|s| binance_um_to_cli_perp(&s.symbol))
            .collect();

        Ok(perp_symbols)
    }

    pub async fn create_listen_key(&self) -> InfraResult<BinanceListenKey> {
        let api_key = self.api_key.as_ref().ok_or(InfraError::ApiNotInitialized)?;

        let listen_key: BinanceListenKey = api_key.send_signed_request(
            &self.client,
            RequestMethod::Post,
            None,
            BINANCE_UM_FUTURES_BASE_URL,
            BINANCE_UM_FUTURES_EXCHANGE_INFO
        ).await?;

        Ok(listen_key)
    }

    pub async fn renew_listen_key(&self) -> InfraResult<BinanceListenKey> {
        let api_key = self.api_key.as_ref().ok_or(InfraError::ApiNotInitialized)?;

        let listen_key: BinanceListenKey = api_key.send_signed_request(
            &self.client,
            RequestMethod::Put,
            None,
            BINANCE_UM_FUTURES_BASE_URL,
            BINANCE_UM_FUTURES_EXCHANGE_INFO
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
                Err(InfraError::Unimplemented)
            },
            WsChannel::Candle(channel) => {
                self.ws_candle_subscription(channel, symbols)
            },
            WsChannel::Trades(_) => {
                Err(InfraError::Unimplemented)
            },
            WsChannel::Tick => {
                Err(InfraError::Unimplemented)
            },
            WsChannel::Lob => {
                Err(InfraError::Unimplemented)
            },
            WsChannel::Other(_) => {
                Err(InfraError::Unimplemented)
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
                    url: BINANCE_UM_FUTURES_WS.to_string(),
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
                    url: format!("{}/{}", BINANCE_UM_FUTURES_WS, listen_key.listenKey),
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
            Some(CandleParam::OneSecond) => BINANCE_CANDLE_SUBSCRIPTIONS[0],
            Some(CandleParam::OneMinute) => BINANCE_CANDLE_SUBSCRIPTIONS[1],
            Some(CandleParam::FiveMinutes) => BINANCE_CANDLE_SUBSCRIPTIONS[2],
            Some(CandleParam::FifteenMinutes) => BINANCE_CANDLE_SUBSCRIPTIONS[3],
            Some(CandleParam::OneHour) => BINANCE_CANDLE_SUBSCRIPTIONS[4],
            Some(CandleParam::FourHours) => BINANCE_CANDLE_SUBSCRIPTIONS[5],
            Some(CandleParam::OneDay) => BINANCE_CANDLE_SUBSCRIPTIONS[6],
            Some(CandleParam::OneWeek) => BINANCE_CANDLE_SUBSCRIPTIONS[7],
            None => BINANCE_CANDLE_SUBSCRIPTIONS[1],
        };

        let msg = self.generate_ws_subscription_msg(channel, symbols);

        Ok(WsSubscription {
            msg: Some(msg),
            url: BINANCE_UM_FUTURES_WS.to_string(),
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
                format!("{}@{}", cli_perp_to_pure_lowercase(symbol), param)
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
