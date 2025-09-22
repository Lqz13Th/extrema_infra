use std::{
    sync::Arc,
    collections::HashMap,
};
use simd_json::from_slice;
use reqwest::Client;
use tracing::error;

use crate::errors::{InfraError, InfraResult};

use crate::market_assets::{
    api_general::RequestMethod,
    base_data::*,
    account_data::*,
};
use crate::task_execution::task_ws::*;

use crate::traits::{
    market_cex::{CexWebsocket, CexPrivateRest, CexPublicRest, MarketCexApi}
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

impl Default for BinanceUmCli {
    fn default() -> Self {
        Self::new(Arc::new(Client::new()))
    }
}

impl MarketCexApi for BinanceUmCli {}


impl CexPublicRest for BinanceUmCli {
    async fn get_live_instruments(&self) -> InfraResult<Vec<String>>{
        self._get_live_instruments().await
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
        assets: Option<&[String]>
    ) -> InfraResult<Vec<BalanceData>> {
        self._get_balance(assets).await
    }
}

impl CexWebsocket for BinanceUmCli {
    async fn get_public_sub_msg(
        &self,
        channel: &WsChannel,
        insts: Option<&[String]>
    ) -> InfraResult<String> {
        self._get_public_sub_msg(channel, insts)
    }

    async fn get_private_sub_msg(
        &self,
        _channel: &WsChannel
    ) -> InfraResult<String> {
        Ok(String::new())
    }

    async fn get_public_connect_msg(
        &self,
        _channel: &WsChannel,
    ) -> InfraResult<String> {
        Ok(BINANCE_UM_FUTURES_WS.into())
    }

    async fn get_private_connect_msg(
        &self,
        _channel: &WsChannel
    ) -> InfraResult<String> {
        let listen_key = self.create_listen_key().await?;
        Ok(format!("{}/{}", BINANCE_UM_FUTURES_WS, listen_key.listenKey))
    }
}


impl BinanceUmCli {
    pub fn new(shared_client: Arc<Client>) -> Self {
        Self {
            client: shared_client,
            api_key: None
        }
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

    async fn _get_balance(
        &self,
        assets: Option<&[String]>
    ) -> InfraResult<Vec<BalanceData>> {
        let api_key = self.api_key.as_ref().ok_or(InfraError::ApiNotInitialized)?;

        let all_balances: Vec<BalanceData> = api_key.send_signed_request(
            &self.client,
            RequestMethod::Get,
            None,
            BINANCE_UM_FUTURES_BASE_URL,
            BINANCE_UM_FUTURES_EXCHANGE_INFO
        ).await?;

        let data = match assets {
            Some(list) if !list.is_empty() => {
                all_balances
                    .into_iter()
                    .filter(|b| list.contains(&b.asset))
                    .collect()
            },
            _ => all_balances,
        };

        Ok(data)
    }

    async fn _get_live_instruments(&self) -> InfraResult<Vec<String>> {
        let url = [BINANCE_UM_FUTURES_BASE_URL, BINANCE_UM_FUTURES_EXCHANGE_INFO].concat();

        let responds = self.client.get(url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: RestExchangeInfoBinanceUM = from_slice(&mut res_bytes)?;

        let data: Vec<String> = res.insts
            .into_iter()
            .filter(|ins| ins.contractType == PERPETUAL && ins.status == TRADING)
            .map(|s| binance_um_to_cli_perp(&s.inst))
            .collect();

        Ok(data)
    }

    fn _get_public_sub_msg(
        &self,
        ws_channel: &WsChannel,
        insts: Option<&[String]>
    ) -> InfraResult<String> {
        match ws_channel {
            WsChannel::Candle(channel) => {
                self._ws_subscribe_candle(channel, insts)
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
            _ => {
                Err(InfraError::Unimplemented)
            },
        }
    }

    fn _ws_subscribe_candle(
        &self,
        candle_param: &Option<CandleParam>,
        insts: Option<&[String]>,
    ) -> InfraResult<String> {
        let interval = candle_param
            .as_ref()
            .map(|p| p.as_str())
            .unwrap_or("1m");

        let channel = format!("kline_{}", interval);

        Ok(ws_subscribe_msg_binance(&channel, insts))
    }
}
