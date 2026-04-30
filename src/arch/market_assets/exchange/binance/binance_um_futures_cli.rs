use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use tracing::error;

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
        market_lob::{LobPrivateRest, LobPublicRest, LobWebsocket, MarketLobApi},
    },
};
use crate::errors::{InfraError, InfraResult};

use super::{
    api_key::{BinanceKey, read_binance_env_key},
    api_utils::*,
    config_assets::*,
    schemas::um_futures_rest::{
        account_balance::RestAccountBalBinanceUM,
        account_position_risk::RestAccountPosRiskBinanceUM,
        exchange_info::RestExchangeInfoBinanceUM, funding_rate::RestFundingRateBinanceUM,
        funding_rate_info::RestFundingInfoBinanceUM, leverage::RestLeverageBinanceUM,
        open_interest_statistics::RestOpenInterestBinanceUM,
        order_history::RestOrderHistoryBinanceUM, premium_index::RestPremiumIndexBinanceUM,
        ticker::RestTickerBinanceUM, trade_order::RestOrderAckBinanceUM,
    },
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

impl MarketLobApi for BinanceUmCli {}

impl LobPublicRest for BinanceUmCli {
    async fn get_tickers(
        &self,
        insts: Option<&[String]>,
        inst_type: Option<InstrumentType>,
    ) -> InfraResult<Vec<TickerData>> {
        self._get_tickers(insts, inst_type).await
    }

    async fn get_instrument_info(
        &self,
        inst_type: InstrumentType,
    ) -> InfraResult<Vec<InstrumentInfo>> {
        self._get_instrument_info(inst_type).await
    }

    async fn get_live_instruments(&self, _inst_type: InstrumentType) -> InfraResult<Vec<String>> {
        self._get_live_instruments().await
    }
}

impl LobPrivateRest for BinanceUmCli {
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
        order_id: Option<&str>,
    ) -> InfraResult<Vec<HistoOrderData>> {
        self._get_order_history(inst, start_time, end_time, limit, order_id)
            .await
    }
}

impl LobWebsocket for BinanceUmCli {
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

    async fn get_public_connect_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        self._get_public_connect_msg(channel)
    }

    async fn get_private_connect_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        self._get_private_connect_msg(channel).await
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

        let res: RestResBinance<BinanceListenKey> = api_key
            .send_signed_request(
                &self.client,
                RequestMethod::Post,
                None,
                BINANCE_UM_FUTURES_BASE_URL,
                BINANCE_UM_FUTURES_LISTEN_KEY,
            )
            .await?;

        let listen_key = res
            .into_vec()?
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError(
                "No UM listen key data returned".into(),
            ))?;

        Ok(listen_key)
    }

    pub async fn renew_listen_key(&self) -> InfraResult<BinanceListenKey> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?;

        let res: RestResBinance<BinanceListenKey> = api_key
            .send_signed_request(
                &self.client,
                RequestMethod::Put,
                None,
                BINANCE_UM_FUTURES_BASE_URL,
                BINANCE_UM_FUTURES_LISTEN_KEY,
            )
            .await?;

        let listen_key = res
            .into_vec()?
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError(
                "No UM listen key data returned".into(),
            ))?;

        Ok(listen_key)
    }

    pub async fn set_leverage(
        &self,
        inst: &str,
        leverage: u32,
    ) -> InfraResult<RestLeverageBinanceUM> {
        let query_string = format!(
            "symbol={}&leverage={}",
            cli_perp_to_pure_uppercase(inst),
            leverage
        );

        let res: RestResBinance<RestLeverageBinanceUM> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Post,
                Some(&query_string),
                BINANCE_UM_FUTURES_BASE_URL,
                BINANCE_UM_FUTURES_CHANGE_LEVERAGE,
            )
            .await?;

        let data = res
            .into_vec()?
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError("No leverage data returned".into()))?;

        Ok(data)
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

        let response = self.client.get(url).send().await?;
        let res: RestResBinance<Vec<Value>> =
            parse_json_response("BinanceUmFutures premium_index_klines", response).await?;

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

    pub async fn get_premium_index(
        &self,
        inst: Option<&str>,
    ) -> InfraResult<Vec<RestPremiumIndexBinanceUM>> {
        let mut url = [
            BINANCE_UM_FUTURES_BASE_URL,
            BINANCE_UM_FUTURES_PREMIUM_INDEX,
        ]
        .concat();

        if let Some(sym) = inst {
            let normalized = cli_perp_to_pure_uppercase(sym);
            url.push_str(&format!("?symbol={}", normalized));
        }

        let response = self.client.get(url).send().await?;
        let res: RestResBinance<RestPremiumIndexBinanceUM> =
            parse_json_response("BinanceUmFutures premium_index", response).await?;

        res.into_vec()
    }

    pub async fn get_funding_rate_live(
        &self,
        inst: Option<&str>,
    ) -> InfraResult<Vec<FundingRateData>> {
        let mut url = [
            BINANCE_UM_FUTURES_BASE_URL,
            BINANCE_UM_FUTURES_PREMIUM_INDEX,
        ]
        .concat();

        if let Some(sym) = inst {
            let normalized = cli_perp_to_pure_uppercase(sym);
            url.push_str(&format!("?symbol={}", normalized));
        }

        let response = self.client.get(url).send().await?;
        let res: RestResBinance<RestPremiumIndexBinanceUM> =
            parse_json_response("BinanceUmFutures funding_rate_live", response).await?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(FundingRateData::from)
            .collect();

        Ok(data)
    }

    pub async fn get_funding_info(&self) -> InfraResult<Vec<FundingRateInfo>> {
        let url = [BINANCE_UM_FUTURES_BASE_URL, BINANCE_UM_FUTURES_FUNDING_INFO].concat();

        let response = self.client.get(url).send().await?;
        let res: RestResBinance<RestFundingInfoBinanceUM> =
            parse_json_response("BinanceUmFutures funding_info", response).await?;

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

        let response = self.client.get(url).send().await?;
        let res: RestResBinance<RestFundingRateBinanceUM> =
            parse_json_response("BinanceUmFutures funding_rate_history", response).await?;

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

        let response = self.client.get(url).send().await?;
        let res: RestResBinance<RestOpenInterestBinanceUM> =
            parse_json_response("BinanceUmFutures open_interest_hist", response).await?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(OpenInterest::from)
            .collect();

        Ok(data)
    }

    async fn _get_tickers(
        &self,
        insts: Option<&[String]>,
        _inst_type: Option<InstrumentType>,
    ) -> InfraResult<Vec<TickerData>> {
        let url = [BINANCE_UM_FUTURES_BASE_URL, BINANCE_UM_FUTURES_TICKERS].concat();

        let response = self.client.get(url).send().await?;
        let res: RestResBinance<RestTickerBinanceUM> =
            parse_json_response("BinanceUmFutures tickers", response).await?;

        let data = res
            .into_vec()?
            .into_iter()
            .filter(|t| match insts {
                Some(list) => list.contains(&binance_fut_inst_to_cli(&t.symbol)),
                None => true,
            })
            .map(TickerData::from)
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

        let response = self.client.get(&url).send().await?;
        let res: RestResBinance<RestExchangeInfoBinanceUM> =
            parse_json_response("BinanceUmFutures instrument_info", response).await?;

        let data = res
            .into_vec()?
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError(
                "No UM exchange info data returned".into(),
            ))?
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

        let response = self.client.get(url).send().await?;
        let res: RestResBinance<RestExchangeInfoBinanceUM> =
            parse_json_response("BinanceUmFutures live_instruments", response).await?;

        let data: Vec<String> = res
            .into_vec()?
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError(
                "No UM exchange info data returned".into(),
            ))?
            .symbols
            .into_iter()
            .filter(|ins| ins.contractType == PERPETUAL && ins.status == TRADING)
            .map(|s| binance_fut_inst_to_cli(&s.symbol))
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

        let data: OrderAckData = res
            .into_vec()?
            .into_iter()
            .map(OrderAckData::from)
            .next()
            .ok_or(InfraError::ApiCliError("No order ack data returned".into()))?;

        Ok(data)
    }

    async fn _get_balance(&self, assets: Option<&[String]>) -> InfraResult<Vec<BalanceData>> {
        let res: RestResBinance<RestAccountBalBinanceUM> = self
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

        let data = res
            .into_vec()?
            .into_iter()
            .filter(|b| match assets {
                Some(list) if !list.is_empty() => list.contains(&b.asset),
                _ => true,
            })
            .map(BalanceData::from)
            .collect();

        Ok(data)
    }

    async fn _get_positions(&self, insts: Option<&[String]>) -> InfraResult<Vec<PositionData>> {
        let res: RestResBinance<RestAccountPosRiskBinanceUM> = self
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

        let data = res
            .into_vec()?
            .into_iter()
            .filter(|t| match insts {
                Some(list) => list.contains(&binance_fut_inst_to_cli(&t.symbol)),
                None => true,
            })
            .map(PositionData::from)
            .collect();

        Ok(data)
    }

    async fn _get_order_history(
        &self,
        inst: &str,
        start_time: Option<u64>,
        end_time: Option<u64>,
        limit: Option<u32>,
        order_id: Option<&str>,
    ) -> InfraResult<Vec<HistoOrderData>> {
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
            .map(HistoOrderData::from)
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

    fn _get_public_connect_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        let url = match channel {
            WsChannel::Candles(_) | WsChannel::Trades(_) => BINANCE_UM_FUTURES_WS_MKT,
            WsChannel::Tick | WsChannel::Lob => BINANCE_UM_FUTURES_WS_PUB,
            _ => return Err(InfraError::Unimplemented),
        };

        Ok(url.into())
    }

    async fn _get_private_connect_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        let events = match channel {
            WsChannel::AccountOrders => "ORDER_TRADE_UPDATE",
            WsChannel::AccountPositions | WsChannel::AccountBalAndPos => "ACCOUNT_UPDATE",
            _ => return Err(InfraError::Unimplemented),
        };

        let listen_key = self.create_listen_key().await?;

        Ok(format!(
            "{}?listenKey={}&events={}",
            BINANCE_UM_FUTURES_WS_PRI, listen_key.listenKey, events
        ))
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
