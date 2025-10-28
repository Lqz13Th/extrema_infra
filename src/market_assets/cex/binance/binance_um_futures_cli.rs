use std::sync::Arc;
use simd_json::from_slice;
use reqwest::Client;
use serde_json::Value;
use tracing::error;

use crate::errors::{InfraError, InfraResult};
use crate::market_assets::{
    cex::binance::um_futures_rest::{
        account_balance::RestAccountBalBinanceUM,
        funding_rate_info::RestFundingInfoBinanceUM,
    },
    api_data::{
        account_data::*,
        price_data::*,
        utils_data::*,
    },
    api_general::RequestMethod,
    base_data::*,
};
use crate::market_assets::api_general::ts_to_micros;
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
    um_futures_rest::exchange_info::RestExchangeInfoBinanceUM,
};


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
            BINANCE_UM_FUTURES_LISTEN_KEY,
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
            BINANCE_UM_FUTURES_LISTEN_KEY,
        ).await?;

        Ok(listen_key)
    }

    pub async fn get_premium_index_klines(
        &self,
        symbol: &str,
        interval: &str,
        limit: Option<u32>,
        start_time: Option<u64>,
        end_time: Option<u64>,
    ) -> InfraResult<Vec<CandleData>> {
        let url = format!(
            "{}{}?symbol={}&interval={}",
            BINANCE_UM_FUTURES_BASE_URL,
            BINANCE_UM_FUTURES_PREMIUM_INDEX_KLINES,
            cli_perp_to_pure_uppercase(symbol),
            interval
        );

        let mut req = self.client.get(&url);

        if let Some(l) = limit {
            req = req.query(&[("limit", l.to_string())]);
        }
        if let Some(start) = start_time {
            req = req.query(&[("startTime", start.to_string())]);
        }
        if let Some(end) = end_time {
            req = req.query(&[("endTime", end.to_string())]);
        }

        let responds = req.send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: Vec<Vec<Value>> = from_slice(&mut res_bytes)?;

        let mut candles = Vec::with_capacity(res.len());
        for entry in res {
            if entry.len() < 5 {
                continue;
            }

            let open_time = ts_to_micros(entry[0].as_u64().unwrap_or_default());
            let open = entry[1].as_str().unwrap_or("0").parse::<f64>().unwrap_or_default();
            let high = entry[2].as_str().unwrap_or("0").parse::<f64>().unwrap_or_default();
            let low = entry[3].as_str().unwrap_or("0").parse::<f64>().unwrap_or_default();
            let close = entry[4].as_str().unwrap_or("0").parse::<f64>().unwrap_or_default();

            candles.push(CandleData::new(symbol, open_time, open, high, low, close));
        }

        Ok(candles)
    }

    pub async fn get_funding_info(&self) -> InfraResult<Vec<FundingRateInfo>> {
        let url = [BINANCE_UM_FUTURES_BASE_URL, BINANCE_UM_FUTURES_FUNDING_INFO].concat();

        let resp = self.client.get(url).send().await?;
        let mut res_bytes = resp.bytes().await?.to_vec();
        let res: Vec<RestFundingInfoBinanceUM> = from_slice(&mut res_bytes)?;

        let data = res
            .into_iter()
            .map(FundingRateInfo::from)
            .collect();

        Ok(data)
    }

    async fn _get_balance(
        &self,
        assets: Option<&[String]>
    ) -> InfraResult<Vec<BalanceData>> {
        let api_key = self.api_key.as_ref().ok_or(InfraError::ApiNotInitialized)?;

        let bal_res: Vec<RestAccountBalBinanceUM> = api_key.send_signed_request(
            &self.client,
            RequestMethod::Get,
            None,
            BINANCE_UM_FUTURES_BASE_URL,
            BINANCE_UM_FUTURES_BALANCE_INFO,
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
        let url = [BINANCE_UM_FUTURES_BASE_URL, BINANCE_UM_FUTURES_EXCHANGE_INFO].concat();

        let responds = self.client.get(url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: RestExchangeInfoBinanceUM = from_slice(&mut res_bytes)?;

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
