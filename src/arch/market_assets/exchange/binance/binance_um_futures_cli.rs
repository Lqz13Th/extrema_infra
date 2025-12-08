use reqwest::Client;
use serde_json::Value;
use simd_json::from_slice;
use std::sync::Arc;
use tracing::error;

use super::{
    api_key::{BinanceKey, read_binance_env_key},
    api_utils::*,
    config_assets::*,
    schemas::um_futures_rest::{
        account_balance::RestAccountBalBinanceUM,
        account_position_risk::RestAccountPosRiskBinanceUM,
        exchange_info::RestExchangeInfoBinanceUM, funding_rate::RestFundingRateBinanceUM,
        funding_rate_info::RestFundingInfoBinanceUM,
        open_interest_statistics::RestOpenInterestBinanceUM,
        order_history::RestOrderHistoryBinanceUM, trade_order::RestOrderAckBinanceUM,
    },
};
use crate::arch::{
    market_assets::{
        api_data::{account_data::*, price_data::*, utils_data::*},
        api_general::*,
        base_data::*,
        exchange::binance::binance_rest_msg::RestResBinance,
    },
    task_execution::task_ws::*,
    traits::{
        conversion::IntoInfraVec,
        market_cex::{CexPrivateRest, CexPublicRest, CexWebsocket, MarketCexApi},
    },
};
use crate::errors::{InfraError, InfraResult};

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
    async fn get_instrument_info(
        &self,
        inst_type: InstrumentType,
    ) -> InfraResult<Vec<InstrumentInfo>> {
        self._get_instrument_info(inst_type).await
    }

    async fn get_live_instruments(&self) -> InfraResult<Vec<String>> {
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
            },
        };
    }

    async fn place_order(&self, order_params: OrderParams) -> InfraResult<OrderAckData> {
        self._place_order(order_params).await
    }

    async fn get_balance(&self, assets: Option<&[String]>) -> InfraResult<Vec<BalanceData>> {
        self._get_balance(assets).await
    }

    async fn get_positions(&self, insts: Option<&[String]>) -> InfraResult<Vec<PositionData>> {
        self._get_positions(insts).await
    }

    async fn get_order_history(
        &self,
        inst: &str,
        start_time: Option<u64>,
        end_time: Option<u64>,
        limit: Option<u32>,
        order_id: Option<u64>,
    ) -> InfraResult<Vec<HistoricalOrder>> {
        self._get_order_history(inst, start_time, end_time, limit, order_id)
            .await
    }
}

impl CexWebsocket for BinanceUmCli {
    async fn get_public_sub_msg(
        &self,
        channel: &WsChannel,
        insts: Option<&[String]>,
    ) -> InfraResult<String> {
        self._get_public_sub_msg(channel, insts)
    }

    async fn get_private_sub_msg(&self, _channel: &WsChannel) -> InfraResult<String> {
        Ok(String::new())
    }

    async fn get_public_connect_msg(&self, _channel: &WsChannel) -> InfraResult<String> {
        Ok(BINANCE_UM_FUTURES_WS.into())
    }

    async fn get_private_connect_msg(&self, _channel: &WsChannel) -> InfraResult<String> {
        let listen_key = self.create_listen_key().await?;
        Ok(format!(
            "{}/{}",
            BINANCE_UM_FUTURES_WS, listen_key.listenKey
        ))
    }
}

impl BinanceUmCli {
    pub fn new(shared_client: Arc<Client>) -> Self {
        Self {
            client: shared_client,
            api_key: None,
        }
    }

    pub async fn create_listen_key(&self) -> InfraResult<BinanceListenKey> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?;

        let listen_key: BinanceListenKey = api_key
            .send_signed_request(
                &self.client,
                RequestMethod::Post,
                None,
                BINANCE_UM_FUTURES_BASE_URL,
                BINANCE_UM_FUTURES_LISTEN_KEY,
            )
            .await?;

        Ok(listen_key)
    }

    pub async fn renew_listen_key(&self) -> InfraResult<BinanceListenKey> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?;

        let listen_key: BinanceListenKey = api_key
            .send_signed_request(
                &self.client,
                RequestMethod::Put,
                None,
                BINANCE_UM_FUTURES_BASE_URL,
                BINANCE_UM_FUTURES_LISTEN_KEY,
            )
            .await?;

        Ok(listen_key)
    }

    pub async fn get_premium_index_klines(
        &self,
        inst: &str,
        interval: &str,
        limit: Option<u32>,
        start_time: Option<u64>,
        end_time: Option<u64>,
    ) -> InfraResult<Vec<CandleData>> {
        let mut url = format!(
            "{}{}?symbol={}&interval={}",
            BINANCE_UM_FUTURES_BASE_URL,
            BINANCE_UM_FUTURES_PREMIUM_INDEX_KLINES,
            cli_perp_to_pure_uppercase(inst),
            interval
        );

        if let Some(l) = limit {
            url.push_str(&format!("&limit={}", l));
        }
        if let Some(start) = start_time {
            url.push_str(&format!("&startTime={}", start));
        }
        if let Some(end) = end_time {
            url.push_str(&format!("&endTime={}", end));
        }

        let responds = self.client.get(url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: RestResBinance<Vec<Value>> = from_slice(&mut res_bytes)?;

        let candles = res
            .into_vec()?
            .into_iter()
            .filter(|entry| entry.len() >= 5)
            .map(|entry| {
                let open_time = ts_to_micros(entry[0].as_u64().unwrap_or_default());
                let open = value_to_f64(&entry[1]);
                let high = value_to_f64(&entry[2]);
                let low = value_to_f64(&entry[3]);
                let close = value_to_f64(&entry[4]);
                CandleData::new(inst, open_time, open, high, low, close)
            })
            .collect();

        Ok(candles)
    }

    pub async fn get_funding_info(&self) -> InfraResult<Vec<FundingRateInfo>> {
        let url = [BINANCE_UM_FUTURES_BASE_URL, BINANCE_UM_FUTURES_FUNDING_INFO].concat();

        let responds = self.client.get(url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: RestResBinance<RestFundingInfoBinanceUM> = from_slice(&mut res_bytes)?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(FundingRateInfo::from)
            .collect();

        Ok(data)
    }

    pub async fn get_funding_rate_history(
        &self,
        inst: Option<&str>,
        limit: Option<u32>,
        start_time: Option<u64>,
        end_time: Option<u64>,
    ) -> InfraResult<Vec<FundingRateData>> {
        let mut url = format!("{}/fapi/v1/fundingRate?", BINANCE_UM_FUTURES_BASE_URL);

        if let Some(s) = inst {
            url.push_str(&format!("symbol={}", cli_perp_to_pure_uppercase(s)));
        }
        if let Some(l) = limit {
            url.push_str(&format!("&limit={}", l));
        }
        if let Some(s) = start_time {
            url.push_str(&format!("&startTime={}", s));
        }
        if let Some(e) = end_time {
            url.push_str(&format!("&endTime={}", e));
        }

        let responds = self.client.get(url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();

        let res: RestResBinance<RestFundingRateBinanceUM> = from_slice(&mut res_bytes)?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(FundingRateData::from)
            .collect();

        Ok(data)
    }

    pub async fn get_open_interest_hist(
        &self,
        inst: &str,
        period: &str,
        limit: Option<u32>,
        start_time: Option<u64>,
        end_time: Option<u64>,
    ) -> InfraResult<Vec<OpenInterest>> {
        let mut url = format!(
            "{}/futures/data/openInterestHist?symbol={}&period={}",
            BINANCE_UM_FUTURES_BASE_URL,
            cli_perp_to_pure_uppercase(inst),
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

        let responds = self.client.get(url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: RestResBinance<RestOpenInterestBinanceUM> = from_slice(&mut res_bytes)?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(OpenInterest::from)
            .collect();

        Ok(data)
    }

    async fn _get_instrument_info(
        &self,
        inst_type: InstrumentType,
    ) -> InfraResult<Vec<InstrumentInfo>> {
        let url = [
            BINANCE_UM_FUTURES_BASE_URL,
            BINANCE_UM_FUTURES_EXCHANGE_INFO,
        ]
        .concat();

        let responds = self.client.get(&url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: RestExchangeInfoBinanceUM = from_slice(&mut res_bytes)?;

        let data = res
            .symbols
            .into_iter()
            .map(InstrumentInfo::from)
            .filter(|i| i.inst_type == inst_type)
            .collect();

        Ok(data)
    }

    async fn _get_live_instruments(&self) -> InfraResult<Vec<String>> {
        let url = [
            BINANCE_UM_FUTURES_BASE_URL,
            BINANCE_UM_FUTURES_EXCHANGE_INFO,
        ]
        .concat();

        let responds = self.client.get(url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: RestExchangeInfoBinanceUM = from_slice(&mut res_bytes)?;

        let data: Vec<String> = res
            .symbols
            .into_iter()
            .filter(|ins| ins.contractType == PERPETUAL && ins.status == TRADING)
            .map(|s| binance_inst_to_cli(&s.symbol))
            .collect();

        Ok(data)
    }

    async fn _place_order(&self, order_params: OrderParams) -> InfraResult<OrderAckData> {
        let mut query_string = format!(
            "symbol={}&side={}&type={}&quantity={}",
            cli_perp_to_pure_uppercase(&order_params.inst),
            match order_params.side {
                OrderSide::BUY => "BUY",
                OrderSide::SELL => "SELL",
                _ => "BUY",
            },
            match order_params.order_type {
                OrderType::Limit => "LIMIT",
                OrderType::Market => "MARKET",
                OrderType::PostOnly => "POST_ONLY",
                OrderType::Fok => "FOK",
                OrderType::Ioc => "IOC",
                OrderType::Unknown => "MARKET",
            },
            order_params.size,
        );

        if let Some(price) = &order_params.price {
            query_string.push_str(&format!("&price={}", price));
        }

        if let Some(ro) = order_params.reduce_only {
            query_string.push_str(&format!("&reduceOnly={}", ro));
        }

        if let Some(ps) = &order_params.position_side {
            let ps_str = match ps {
                PositionSide::Long => "LONG",
                PositionSide::Short => "SHORT",
                PositionSide::Both => "BOTH",
                PositionSide::Unknown => "BOTH",
            };
            query_string.push_str(&format!("&positionSide={}", ps_str));
        }

        if let Some(tif) = &order_params.time_in_force {
            let tif_str = match tif {
                TimeInForce::GTC => "GTC",
                TimeInForce::IOC => "IOC",
                TimeInForce::FOK => "FOK",
                TimeInForce::GTD => "GTD",
                TimeInForce::Unknown => "GTC",
            };
            query_string.push_str(&format!("&timeInForce={}", tif_str));
        }

        if let Some(cid) = &order_params.client_order_id {
            query_string.push_str(&format!("&newClientOrderId={}", cid));
        }

        for (k, v) in &order_params.extra {
            query_string.push_str(&format!("&{}={}", k, v));
        }
        println!("{}", query_string);
        let res: RestResBinance<RestOrderAckBinanceUM> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Post,
                Some(&query_string),
                BINANCE_UM_FUTURES_BASE_URL,
                BINANCE_UM_FUTURES_PLACE_ORDER_INFO,
            )
            .await?;

        tracing::warn!("{:#?}", res);

        let data: OrderAckData = res
            .into_vec()?
            .into_iter()
            .map(OrderAckData::from)
            .next()
            .ok_or(InfraError::ApiCliError("No order ack data returned".into()))?;

        Ok(data)
    }

    async fn _get_balance(&self, assets: Option<&[String]>) -> InfraResult<Vec<BalanceData>> {
        let bal_res: RestResBinance<RestAccountBalBinanceUM> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                None,
                BINANCE_UM_FUTURES_BASE_URL,
                BINANCE_UM_FUTURES_BALANCE_INFO,
            )
            .await?;

        let filtered_res = match assets {
            Some(list) if !list.is_empty() => bal_res
                .into_vec()?
                .into_iter()
                .filter(|b| list.contains(&b.asset))
                .collect(),
            _ => bal_res.into_vec()?,
        };

        let data = filtered_res.into_iter().map(BalanceData::from).collect();

        Ok(data)
    }

    async fn _get_positions(&self, insts: Option<&[String]>) -> InfraResult<Vec<PositionData>> {
        let pos_res: RestResBinance<RestAccountPosRiskBinanceUM> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                None,
                BINANCE_UM_FUTURES_BASE_URL,
                BINANCE_UM_FUTURES_POSITION_RISK_INFO,
            )
            .await?;

        let filtered_res = match insts {
            Some(list) if !list.is_empty() => pos_res
                .into_vec()?
                .into_iter()
                .filter(|p| list.contains(&p.symbol))
                .collect(),
            _ => pos_res.into_vec()?,
        };

        let data = filtered_res.into_iter().map(PositionData::from).collect();

        Ok(data)
    }

    async fn _get_order_history(
        &self,
        inst: &str,
        start_time: Option<u64>,
        end_time: Option<u64>,
        limit: Option<u32>,
        order_id: Option<u64>,
    ) -> InfraResult<Vec<HistoricalOrder>> {
        let mut query_string = format!("symbol={}", cli_perp_to_pure_uppercase(inst));

        if let Some(oid) = order_id {
            query_string.push_str(&format!("&orderId={}", oid));
        }

        if let Some(start) = start_time {
            query_string.push_str(&format!("&startTime={}", start));
        }

        if let Some(end) = end_time {
            query_string.push_str(&format!("&endTime={}", end));
        }

        if let Some(l) = limit {
            query_string.push_str(&format!("&limit={}", l));
        }

        let res: RestResBinance<RestOrderHistoryBinanceUM> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                Some(&query_string),
                BINANCE_UM_FUTURES_BASE_URL,
                BINANCE_UM_FUTURES_ALL_ORDERS,
            )
            .await?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(HistoricalOrder::from)
            .collect();

        Ok(data)
    }

    fn _get_public_sub_msg(
        &self,
        ws_channel: &WsChannel,
        insts: Option<&[String]>,
    ) -> InfraResult<String> {
        match ws_channel {
            WsChannel::Candles(channel) => self._ws_subscribe_candle(channel, insts),
            WsChannel::Trades(_) => self._ws_subscribe_aggtrade(insts),
            WsChannel::Tick => Err(InfraError::Unimplemented),
            WsChannel::Lob => Err(InfraError::Unimplemented),
            _ => Err(InfraError::Unimplemented),
        }
    }

    fn _ws_subscribe_candle(
        &self,
        candle_param: &Option<CandleParam>,
        insts: Option<&[String]>,
    ) -> InfraResult<String> {
        let interval = candle_param.as_ref().map(|p| p.as_str()).unwrap_or("1m");

        let channel = format!("kline_{}", interval);

        Ok(ws_subscribe_msg_binance(&channel, insts))
    }

    fn _ws_subscribe_aggtrade(&self, insts: Option<&[String]>) -> InfraResult<String> {
        Ok(ws_subscribe_msg_binance("aggTrade", insts))
    }
}
