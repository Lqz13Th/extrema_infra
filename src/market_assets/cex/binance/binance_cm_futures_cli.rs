use std::sync::Arc;
use simd_json::from_slice;
use reqwest::Client;
use tracing::error;

use crate::errors::{InfraError, InfraResult};
use crate::market_assets::{
    api_data::{
        account_data::*,
        utils_data::*,
    },
    api_general::RequestMethod,
    base_data::*,
};
use crate::market_assets::cex::binance::cm_futures_rest::account_balance::RestAccountBalBinanceCM;
use crate::task_execution::task_ws::*;
use crate::traits::market_cex::{
    CexPrivateRest,
    CexPublicRest,
    CexWebsocket,
    MarketCexApi,
};

use super::{
    api_key::{read_binance_env_key, BinanceKey},
    api_utils::*,
    config_assets::*,
    cm_futures_rest::{
        exchange_info::RestExchangeInfoBinanceCM,
        open_interest_statistics::RestOpenInterestBinanceCM,
    },
};


#[derive(Clone, Debug)]
pub struct BinanceCmCli {
    pub client: Arc<Client>,
    pub api_key: Option<BinanceKey>,
}

impl Default for BinanceCmCli {
    fn default() -> Self {
        Self::new(Arc::new(Client::new()))
    }
}

impl MarketCexApi for BinanceCmCli {}


impl CexPublicRest for BinanceCmCli {
    async fn get_live_instruments(&self) -> InfraResult<Vec<String>>{
        self._get_live_instruments().await
    }
}

impl CexPrivateRest for BinanceCmCli {
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

impl CexWebsocket for BinanceCmCli {
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
        Ok(BINANCE_CM_FUTURES_WS.into())
    }

    async fn get_private_connect_msg(
        &self,
        _channel: &WsChannel
    ) -> InfraResult<String> {
        let listen_key = self.create_listen_key().await?;
        Ok(format!("{}/{}", BINANCE_CM_FUTURES_WS, listen_key.listenKey))
    }
}


impl BinanceCmCli {
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
            BINANCE_CM_FUTURES_BASE_URL,
            BINANCE_CM_FUTURES_LISTEN_KEY,
        ).await?;

        Ok(listen_key)
    }

    pub async fn renew_listen_key(&self) -> InfraResult<BinanceListenKey> {
        let api_key = self.api_key.as_ref().ok_or(InfraError::ApiNotInitialized)?;

        let listen_key: BinanceListenKey = api_key.send_signed_request(
            &self.client,
            RequestMethod::Put,
            None,
            BINANCE_CM_FUTURES_BASE_URL,
            BINANCE_CM_FUTURES_LISTEN_KEY,
        ).await?;

        Ok(listen_key)
    }

    pub async fn get_open_interest_hist(
        &self,
        symbol: &str,
        period: &str,
        limit: Option<u32>,
        start_time: Option<u64>,
        end_time: Option<u64>,
    ) -> InfraResult<Vec<OpenInterest>> {
        let mut url = format!(
            "{}/futures/data/openInterestHist?symbol={}&period={}",
            BINANCE_UM_FUTURES_BASE_URL,
            symbol,
            period,
        );

        if let Some(l) = limit {
            url.push_str(&format!("&limit={}", l));
        }
        if let Some(s) = start_time {
            url.push_str(&format!("&startTime={}", s));
        }
        if let Some(e) = end_time {
            url.push_str(&format!("&endTime={}", e));
        }

        let responds = self.client.get(&url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: Vec<RestOpenInterestBinanceCM> = from_slice(&mut res_bytes)?;

        let data = res
            .into_iter()
            .map(OpenInterest::from)
            .collect();

        Ok(data)
    }

    async fn _get_balance(
        &self,
        assets: Option<&[String]>
    ) -> InfraResult<Vec<BalanceData>> {
        let api_key = self.api_key.as_ref().ok_or(InfraError::ApiNotInitialized)?;

        let bal_res: Vec<RestAccountBalBinanceCM> = api_key.send_signed_request(
            &self.client,
            RequestMethod::Get,
            None,
            BINANCE_CM_FUTURES_BASE_URL,
            BINANCE_CM_FUTURES_BALANCE_INFO,
        ).await?;

        let filtered_res = match assets {
            Some(list) if !list.is_empty() => {
                bal_res
                    .into_iter()
                    .filter(|b| list.contains(&b.asset))
                    .collect()
            },
            _ => bal_res,
        };

        let data: Vec<BalanceData> = filtered_res
            .into_iter()
            .map(BalanceData::from)
            .collect();

        Ok(data)
    }

    async fn _get_live_instruments(&self) -> InfraResult<Vec<String>> {
        let url = [BINANCE_CM_FUTURES_BASE_URL, BINANCE_CM_FUTURES_EXCHANGE_INFO].concat();

        let responds = self.client.get(url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: RestExchangeInfoBinanceCM = from_slice(&mut res_bytes)?;

        let data: Vec<String> = res.symbols
            .into_iter()
            .filter(|ins| ins.contractType == PERPETUAL && ins.status == TRADING)
            .map(|s| binance_inst_to_cli(&s.symbol))
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
