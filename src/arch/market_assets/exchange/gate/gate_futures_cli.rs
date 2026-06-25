use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use tracing::error;

use crate::arch::{
    market_assets::{
        api_data::{
            account_data::{HistoOrderData, OrderAckData, PositionData},
            price_data::{CandleData, OrderBookData, TickerData},
            utils_data::{FundingRateData, FundingRateInfo, InstrumentInfo},
        },
        api_general::{
            OrderParams, RequestMethod, get_seconds_timestamp, parse_json_response, value_to_f64,
        },
        base_data::{InstrumentType, MarginMode, OrderSide, OrderType, SUBSCRIBE_LOWER},
    },
    task_execution::task_ws::{CandleParam, LobParam, WsChannel},
    traits::{
        conversion::IntoInfraVec,
        market_lob::{LobPrivateRest, LobPublicRest, LobWebsocket, MarketLobApi},
    },
};
use crate::errors::{InfraError, InfraResult};

use super::{
    api_key::{GateKey, read_gate_env_key},
    api_utils::*,
    config_assets::*,
    gate_rest_msg::RestResGate,
    schemas::futures_rest::{
        account_position::RestAccountPosGateFutures, candle::RestCandleGateFutures,
        contract_futures::RestContractGateFutures, funding_rate::RestFundingRateGateFutures,
        order::RestFuturesOrderGateFutures, order_history::RestFuturesOrderHistoryGateFutures,
        orderbook::RestOrderBookGateFutures, ticker::RestTickerGateFutures,
    },
};

#[derive(Clone, Debug)]
pub struct GateFuturesCli {
    pub client: Arc<Client>,
    pub api_key: Option<GateKey>,
}

impl Default for GateFuturesCli {
    fn default() -> Self {
        Self::new(Arc::new(Client::new()))
    }
}

impl MarketLobApi for GateFuturesCli {}

impl LobPublicRest for GateFuturesCli {
    async fn get_tickers(
        &self,
        insts: Option<&[String]>,
        inst_type: Option<InstrumentType>,
    ) -> InfraResult<Vec<TickerData>> {
        self._get_tickers(insts, inst_type).await
    }

    async fn get_candles(
        &self,
        inst: &str,
        _inst_type: InstrumentType,
        interval: CandleParam,
        limit: Option<u32>,
        start_time_us: Option<u64>,
        end_time_us: Option<u64>,
    ) -> InfraResult<Vec<CandleData>> {
        self._get_candles(inst, interval, limit, start_time_us, end_time_us)
            .await
    }

    async fn get_orderbook(
        &self,
        inst: &str,
        inst_type: InstrumentType,
        depth: usize,
    ) -> InfraResult<OrderBookData> {
        self._get_orderbook(inst, inst_type, depth).await
    }

    async fn get_instrument_info(
        &self,
        inst_type: InstrumentType,
    ) -> InfraResult<Vec<InstrumentInfo>> {
        self._get_instrument_info(inst_type).await
    }

    async fn get_live_instruments(&self, inst_type: InstrumentType) -> InfraResult<Vec<String>> {
        self._get_live_instruments(inst_type).await
    }
}

impl LobPrivateRest for GateFuturesCli {
    fn init_api_key(&mut self) {
        match read_gate_env_key() {
            Ok(gate_key) => {
                self.api_key = Some(gate_key);
            },
            Err(e) => {
                error!("Failed to read GATE env key: {:?}", e);
            },
        };
    }

    async fn place_order(&self, order_params: OrderParams) -> InfraResult<OrderAckData> {
        self._place_order(order_params).await
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

impl LobWebsocket for GateFuturesCli {
    async fn get_public_sub_msg(
        &self,
        channel: &WsChannel,
        insts: Option<&[String]>,
    ) -> InfraResult<String> {
        self._get_public_sub_msg(channel, insts)
    }

    async fn get_private_sub_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        self._get_private_sub_msg(channel)
    }

    async fn get_public_connect_msg(&self, _channel: &WsChannel) -> InfraResult<String> {
        Ok(GATE_FUTURES_WS_USDT.into())
    }

    async fn get_private_connect_msg(&self, _channel: &WsChannel) -> InfraResult<String> {
        Ok(GATE_FUTURES_WS_USDT.into())
    }
}

impl GateFuturesCli {
    pub fn new(shared_client: Arc<Client>) -> Self {
        Self {
            client: shared_client,
            api_key: None,
        }
    }

    pub fn ws_subscribe_private(&self, channel: &str) -> InfraResult<String> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?;

        let timestamp = get_seconds_timestamp();
        let auth = api_key.ws_auth(channel, SUBSCRIBE_LOWER, timestamp)?;
        let payload = vec![api_key.user_id.clone(), "!all".into()];

        let msg = json!({
            "time": timestamp,
            "channel": channel,
            "event": SUBSCRIBE_LOWER,
            "payload": payload,
            "auth": auth,
        });

        Ok(msg.to_string())
    }

    pub async fn get_funding_rate_history(
        &self,
        settle: &str,
        inst: &str,
        limit: Option<u32>,
        start_time: Option<u64>,
        end_time: Option<u64>,
    ) -> InfraResult<Vec<FundingRateData>> {
        let endpoint = GATE_FUTURES_FUNDING_RATE.replace("{settle}", settle);

        let mut params: Vec<String> = Vec::new();
        params.push(format!("contract={}", cli_perp_to_gate_inst(inst)));
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }
        if let Some(s) = start_time {
            params.push(format!("from={}", s));
        }
        if let Some(e) = end_time {
            params.push(format!("to={}", e));
        }

        let url = if params.is_empty() {
            [GATE_BASE_URL, &endpoint].concat()
        } else {
            format!("{}{}?{}", GATE_BASE_URL, endpoint, params.join("&"))
        };

        let response = self.client.get(url).send().await?;
        let res: RestResGate<RestFundingRateGateFutures> =
            parse_json_response("GateFutures funding_rate_history", response).await?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(|entry| FundingRateData::from((entry, inst)))
            .collect();

        Ok(data)
    }

    pub async fn set_leverage(
        &self,
        inst: &str,
        leverage: u32,
        margin_mode: MarginMode,
    ) -> InfraResult<RestAccountPosGateFutures> {
        let settle = infer_settle_from_inst(inst);
        let contract = cli_perp_to_gate_inst(inst);
        let endpoint = GATE_FUTURES_SET_LEVERAGE
            .replace("{settle}", &settle)
            .replace("{contract}", &contract);

        let mut params = Vec::new();
        match margin_mode {
            MarginMode::Cross => {
                params.push("leverage=0".to_string());
                params.push(format!("cross_leverage_limit={}", leverage));
            },
            MarginMode::Isolated => {
                params.push(format!("leverage={}", leverage));
            },
            MarginMode::Unknown => {
                return Err(InfraError::ApiCliError(format!(
                    "unsupported Gate futures margin_mode: {:?}",
                    margin_mode
                )));
            },
        }

        let res: RestResGate<RestAccountPosGateFutures> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Post,
                Some(&params.join("&")),
                None,
                GATE_BASE_URL,
                &endpoint,
            )
            .await?;

        let data = res
            .into_vec()?
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError("No position data returned".into()))?;

        Ok(data)
    }

    pub async fn get_funding_rate_info(
        &self,
        settle: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> InfraResult<Vec<FundingRateInfo>> {
        let endpoint = GATE_FUTURES_CONTRACTS.replace("{settle}", settle);

        let mut params: Vec<String> = Vec::new();
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }
        if let Some(o) = offset {
            params.push(format!("offset={}", o));
        }

        let url = if params.is_empty() {
            [GATE_BASE_URL, &endpoint].concat()
        } else {
            format!("{}{}?{}", GATE_BASE_URL, endpoint, params.join("&"))
        };

        let response = self.client.get(url).send().await?;
        let res: RestResGate<RestContractGateFutures> =
            parse_json_response("GateFutures funding_rate_info", response).await?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(FundingRateInfo::from)
            .collect();

        Ok(data)
    }

    pub async fn get_funding_rate_live_all(
        &self,
        settle: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> InfraResult<Vec<FundingRateData>> {
        let endpoint = GATE_FUTURES_CONTRACTS.replace("{settle}", settle);

        let mut params: Vec<String> = Vec::new();
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }
        if let Some(o) = offset {
            params.push(format!("offset={}", o));
        }

        let url = if params.is_empty() {
            [GATE_BASE_URL, &endpoint].concat()
        } else {
            format!("{}{}?{}", GATE_BASE_URL, endpoint, params.join("&"))
        };

        let response = self.client.get(url).send().await?;
        let res: RestResGate<RestContractGateFutures> =
            parse_json_response("GateFutures funding_rate_live_all", response).await?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(FundingRateData::from)
            .collect();

        Ok(data)
    }

    pub async fn get_funding_rate_live(
        &self,
        settle: &str,
        inst: &str,
    ) -> InfraResult<Vec<FundingRateData>> {
        let endpoint = GATE_FUTURES_CONTRACT
            .replace("{settle}", settle)
            .replace("{contract}", &cli_perp_to_gate_inst(inst));

        let url = [GATE_BASE_URL, &endpoint].concat();
        let response = self.client.get(url).send().await?;
        let res: RestResGate<RestContractGateFutures> =
            parse_json_response("GateFutures funding_rate_live", response).await?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(FundingRateData::from)
            .collect();

        Ok(data)
    }

    async fn _get_futures_contracts(
        &self,
        settle: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> InfraResult<Vec<RestContractGateFutures>> {
        let endpoint = GATE_FUTURES_CONTRACTS.replace("{settle}", settle);

        let mut params: Vec<String> = Vec::new();
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }
        if let Some(o) = offset {
            params.push(format!("offset={}", o));
        }

        let url = if params.is_empty() {
            [GATE_BASE_URL, &endpoint].concat()
        } else {
            format!("{}{}?{}", GATE_BASE_URL, endpoint, params.join("&"))
        };

        let response = self.client.get(url).send().await?;
        let res: RestResGate<RestContractGateFutures> =
            parse_json_response("GateFutures futures_contracts", response).await?;

        res.into_vec()
    }

    async fn _get_tickers(
        &self,
        insts: Option<&[String]>,
        _inst_type: Option<InstrumentType>,
    ) -> InfraResult<Vec<TickerData>> {
        let mut data: Vec<TickerData> = Vec::new();

        for settle in ["usdt", "btc"] {
            let endpoint = GATE_FUTURES_TICKERS.replace("{settle}", settle);
            let url = [GATE_BASE_URL, &endpoint].concat();

            let response = self.client.get(url).send().await?;
            let res: RestResGate<RestTickerGateFutures> =
                parse_json_response("GateFutures tickers", response).await?;

            data.extend(
                res.into_vec()?
                    .into_iter()
                    .filter(|t| match insts {
                        Some(list) => list.contains(&gate_fut_inst_to_cli(&t.contract)),
                        None => true,
                    })
                    .map(TickerData::from),
            );
        }

        Ok(data)
    }

    async fn _get_candles(
        &self,
        inst: &str,
        interval: CandleParam,
        limit: Option<u32>,
        start_time_us: Option<u64>,
        end_time_us: Option<u64>,
    ) -> InfraResult<Vec<CandleData>> {
        let settle = infer_settle_from_inst(inst);
        let endpoint = GATE_FUTURES_CANDLESTICKS.replace("{settle}", &settle);
        let mut params = vec![
            format!("contract={}", cli_perp_to_gate_inst(inst)),
            format!("interval={}", interval.as_str()),
        ];
        let has_time_window = start_time_us.is_some() || end_time_us.is_some();
        if !has_time_window && let Some(limit) = limit {
            params.push(format!("limit={limit}"));
        }
        if let Some(start_time_us) = start_time_us {
            params.push(format!("from={}", start_time_us / 1_000_000));
        }
        if let Some(end_time_us) = end_time_us {
            params.push(format!("to={}", end_time_us / 1_000_000));
        }

        let url = format!("{}{}?{}", GATE_BASE_URL, endpoint, params.join("&"));

        let response = self.client.get(url).send().await?;
        let res: RestResGate<RestCandleGateFutures> =
            parse_json_response("GateFutures candles", response).await?;

        let mut data: Vec<CandleData> = res
            .into_vec()?
            .into_iter()
            .map(|entry| entry.into_candle_data(inst))
            .filter(|entry| start_time_us.is_none_or(|start| entry.timestamp >= start))
            .filter(|entry| end_time_us.is_none_or(|end| entry.timestamp <= end))
            .collect();
        data.sort_by_key(|candle| candle.timestamp);

        if has_time_window
            && let Some(limit) = limit
            && data.len() > limit as usize
        {
            data.drain(..data.len() - limit as usize);
        }

        Ok(data)
    }

    async fn _get_orderbook(
        &self,
        inst: &str,
        inst_type: InstrumentType,
        depth: usize,
    ) -> InfraResult<OrderBookData> {
        if !matches!(
            inst_type,
            InstrumentType::Perpetual | InstrumentType::Futures
        ) {
            return Err(InfraError::ApiCliError(format!(
                "Gate futures orderbook supports futures/perpetual instruments only, got {:?}",
                inst_type
            )));
        }

        let settle = infer_settle_from_inst(inst);
        let endpoint = GATE_FUTURES_ORDER_BOOK.replace("{settle}", &settle);
        let depth = gate_lob_depth(&nonzero_depth_u16(depth)?)?;
        let params = [
            format!("contract={}", cli_perp_to_gate_inst(inst)),
            format!("limit={depth}"),
        ];
        let url = format!("{}{}?{}", GATE_BASE_URL, endpoint, params.join("&"));

        let response = self.client.get(url).send().await?;
        let res: RestResGate<RestOrderBookGateFutures> =
            parse_json_response("GateFutures orderbook", response).await?;

        res.into_vec()?
            .into_iter()
            .next()
            .map(|entry| entry.into_orderbook_data(inst))
            .ok_or_else(|| {
                InfraError::ApiCliError("No Gate futures orderbook data returned".into())
            })
    }

    async fn _get_instrument_info(
        &self,
        _inst_type: InstrumentType,
    ) -> InfraResult<Vec<InstrumentInfo>> {
        let mut data: Vec<InstrumentInfo> = Vec::new();

        for settle in ["usdt", "btc"] {
            let contracts = self._get_futures_contracts(settle, None, None).await?;
            data.extend(contracts.into_iter().map(InstrumentInfo::from));
        }

        Ok(data)
    }

    async fn _get_live_instruments(&self, _inst_type: InstrumentType) -> InfraResult<Vec<String>> {
        let mut data: Vec<String> = Vec::new();

        let now_secs = get_seconds_timestamp();

        for settle in ["usdt", "btc"] {
            let contracts = self._get_futures_contracts(settle, None, None).await?;
            data.extend(
                contracts
                    .into_iter()
                    .filter(|c| c.is_live(now_secs))
                    .map(|c| gate_fut_inst_to_cli(&c.name)),
            );
        }

        Ok(data)
    }

    async fn _place_order(&self, order_params: OrderParams) -> InfraResult<OrderAckData> {
        let mut extra = order_params.extra;
        let gate_channel_id = take_gate_channel_id(&mut extra)?;
        let settle = extra
            .remove("settle")
            .unwrap_or_else(|| infer_settle_from_inst(&order_params.inst));

        let contract = cli_perp_to_gate_inst(&order_params.inst);
        let size_val: i64 = order_params
            .size
            .parse()
            .map_err(|_| InfraError::ApiCliError("Invalid order size".into()))?;
        let signed_size = match order_params.side {
            OrderSide::SELL => -size_val.abs(),
            _ => size_val.abs(),
        };

        let mut body = json!({
            "contract": contract,
            "size": signed_size,
        });

        let tif = match order_params.order_type {
            OrderType::PostOnly => Some("poc"),
            OrderType::Fok => Some("fok"),
            OrderType::Ioc => Some("ioc"),
            OrderType::Market => Some("ioc"),
            _ => None,
        };

        if matches!(order_params.order_type, OrderType::Market) {
            body["price"] = json!("0");
            body["tif"] = json!(tif.unwrap_or("ioc"));
        } else {
            let price = order_params.price.ok_or(InfraError::ApiCliError(
                "Price required for limit order".into(),
            ))?;
            body["price"] = json!(price);
            let tif_val = tif.unwrap_or("gtc");
            body["tif"] = json!(tif_val);
        }

        if let Some(reduce_only) = order_params.reduce_only {
            body["reduce_only"] = json!(reduce_only);
        }

        if let Some(cl_id) = order_params.client_order_id {
            body["text"] = json!(normalize_gate_text(&cl_id));
        }

        for (k, v) in extra {
            body[k] = json!(v);
        }

        let endpoint = GATE_FUTURES_ORDERS.replace("{settle}", &settle);
        let res: RestResGate<RestFuturesOrderGateFutures> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_post_request_with_channel_id(
                &self.client,
                None,
                Some(&body.to_string()),
                GATE_BASE_URL,
                &endpoint,
                gate_channel_id.as_deref(),
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

    async fn _get_positions(&self, insts: Option<&[String]>) -> InfraResult<Vec<PositionData>> {
        let settles: Vec<String> = if let Some(list) = insts {
            let mut s = Vec::new();
            for inst in list {
                let settle = infer_settle_from_inst(inst);
                if !s.contains(&settle) {
                    s.push(settle);
                }
            }
            if s.is_empty() { vec!["usdt".into()] } else { s }
        } else {
            vec!["usdt".into(), "btc".into()]
        };

        let mut data: Vec<PositionData> = Vec::new();
        for settle in settles {
            let endpoint = GATE_FUTURES_POSITIONS.replace("{settle}", &settle);
            let res: RestResGate<RestAccountPosGateFutures> = self
                .api_key
                .as_ref()
                .ok_or(InfraError::ApiCliNotInitialized)?
                .send_signed_request(
                    &self.client,
                    RequestMethod::Get,
                    None,
                    None,
                    GATE_BASE_URL,
                    &endpoint,
                )
                .await?;

            let pos_raw = match res {
                RestResGate::Error { label, message }
                    if label == "USER_NOT_FOUND"
                        || message
                            .contains("please transfer funds first to create futures account") =>
                {
                    continue;
                },
                other => other,
            };

            data.extend(
                pos_raw
                    .into_vec()?
                    .into_iter()
                    .filter(|p| value_to_f64(&p.size) != 0.0)
                    .filter(|t| match insts {
                        Some(list) => list.contains(&gate_fut_inst_to_cli(&t.contract)),
                        None => true,
                    })
                    .map(PositionData::from),
            );
        }

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
        let settle = infer_settle_from_inst(inst);
        let contract = cli_perp_to_gate_inst(inst);

        let (endpoint, query_string) = if let Some(order_id) = order_id {
            (
                GATE_FUTURES_ORDER
                    .replace("{settle}", &settle)
                    .replace("{order_id}", order_id),
                None,
            )
        } else {
            let mut query = format!("status=finished&contract={}", contract);
            if let Some(start_time) = start_time {
                query.push_str(&format!("&from={}", start_time));
            }
            if let Some(end_time) = end_time {
                query.push_str(&format!("&to={}", end_time));
            }
            if let Some(limit) = limit {
                query.push_str(&format!("&limit={}", limit));
            }
            (
                GATE_FUTURES_ORDERS.replace("{settle}", &settle),
                Some(query),
            )
        };

        let res: RestResGate<RestFuturesOrderHistoryGateFutures> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                query_string.as_deref(),
                None,
                GATE_BASE_URL,
                &endpoint,
            )
            .await?;

        let data: Vec<HistoOrderData> = res
            .into_vec()?
            .into_iter()
            .filter(|order| order.contract == contract)
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
            WsChannel::Trades(_) => self._ws_subscribe_trades(insts),
            WsChannel::Lob(lob_param) => self._ws_subscribe_lob(lob_param, insts),
            _ => Err(InfraError::Unimplemented),
        }
    }

    fn _ws_subscribe_candle(
        &self,
        candle_param: &Option<CandleParam>,
        insts: Option<&[String]>,
    ) -> InfraResult<String> {
        let interval = candle_param.as_ref().map(|p| p.as_str()).unwrap_or("1m");
        let contract = gate_first_contract(insts)?;
        let payload = vec![interval.into(), contract];
        Ok(ws_subscribe_msg_gate_futures(
            GATE_WS_FUTURES_CANDLES,
            payload,
        ))
    }

    fn _ws_subscribe_trades(&self, insts: Option<&[String]>) -> InfraResult<String> {
        let contracts = gate_contracts_from_insts(insts)?;
        Ok(ws_subscribe_msg_gate_futures(
            GATE_WS_FUTURES_TRADES,
            contracts,
        ))
    }

    fn _ws_subscribe_lob(
        &self,
        lob_param: &Option<LobParam>,
        insts: Option<&[String]>,
    ) -> InfraResult<String> {
        match lob_param {
            Some(LobParam::Bbo { frequency }) => {
                gate_lob_bbo_frequency(frequency)?;
                let contracts = gate_contracts_from_insts(insts)?;
                Ok(ws_subscribe_msg_gate_futures(
                    GATE_WS_FUTURES_BOOK_TICKER,
                    contracts,
                ))
            },
            Some(LobParam::Snapshot { depth, frequency }) => {
                gate_lob_snapshot_frequency(frequency)?;
                let contract = gate_first_contract(insts)?;
                let depth = gate_lob_depth(depth)?;
                Ok(ws_subscribe_msg_gate_futures(
                    GATE_WS_FUTURES_ORDER_BOOK,
                    vec![contract, depth.to_string(), "0".into()],
                ))
            },
            None => {
                let contract = gate_first_contract(insts)?;
                Ok(ws_subscribe_msg_gate_futures(
                    GATE_WS_FUTURES_ORDER_BOOK_UPDATE,
                    vec![contract, "100ms".into(), "20".into()],
                ))
            },
            Some(LobParam::Incremental { depth, frequency }) => {
                let contract = gate_first_contract(insts)?;
                let depth = gate_lob_depth(depth)?;
                let frequency = gate_lob_update_frequency(frequency, depth)?;
                Ok(ws_subscribe_msg_gate_futures(
                    GATE_WS_FUTURES_ORDER_BOOK_UPDATE,
                    vec![contract, frequency.into(), depth.to_string()],
                ))
            },
        }
    }

    fn _get_private_sub_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        let topic = match channel {
            WsChannel::AccountOrders => GATE_WS_FUTURES_ORDERS,
            WsChannel::AccountPositions => GATE_WS_FUTURES_POSITIONS,
            _ => return Err(InfraError::Unimplemented),
        };
        self.ws_subscribe_private(topic)
    }
}

#[cfg(test)]
mod tests {
    use crate::arch::{
        market_assets::exchange::gate::gate_futures_cli::GateFuturesCli,
        task_execution::task_ws::{LobFrequency, LobParam, WsChannel},
        traits::market_lob::LobWebsocket,
    };

    #[tokio::test]
    async fn builds_gate_lob_subscription_messages() {
        let cli = GateFuturesCli::default();
        let insts = vec!["BTC_USDT_PERP".to_string(), "ETH_USDT_PERP".to_string()];

        let bbo = cli
            .get_public_sub_msg(
                &WsChannel::Lob(Some(LobParam::Bbo {
                    frequency: Some(LobFrequency::Realtime),
                })),
                Some(&insts),
            )
            .await
            .unwrap();
        assert!(bbo.contains("\"channel\":\"futures.book_ticker\""));
        assert!(bbo.contains("\"BTC_USDT\""));
        assert!(bbo.contains("\"ETH_USDT\""));

        let snapshot = cli
            .get_public_sub_msg(
                &WsChannel::Lob(Some(LobParam::Snapshot {
                    depth: Some(50),
                    frequency: None,
                })),
                Some(&insts),
            )
            .await
            .unwrap();
        assert!(snapshot.contains("\"channel\":\"futures.order_book\""));
        assert!(snapshot.contains("\"payload\":[\"BTC_USDT\",\"50\",\"0\"]"));

        let incremental = cli
            .get_public_sub_msg(
                &WsChannel::Lob(Some(LobParam::Incremental {
                    depth: Some(20),
                    frequency: Some(LobFrequency::Ms20),
                })),
                Some(&insts),
            )
            .await
            .unwrap();
        assert!(incremental.contains("\"channel\":\"futures.order_book_update\""));
        assert!(incremental.contains("\"payload\":[\"BTC_USDT\",\"20ms\",\"20\"]"));
    }

    #[tokio::test]
    async fn rejects_unsupported_gate_lob_subscription_params() {
        let cli = GateFuturesCli::default();
        let insts = vec!["BTC_USDT_PERP".to_string()];

        let err = cli
            .get_public_sub_msg(
                &WsChannel::Lob(Some(LobParam::Snapshot {
                    depth: Some(20),
                    frequency: Some(LobFrequency::Ms100),
                })),
                Some(&insts),
            )
            .await;
        assert!(err.is_err());

        let err = cli
            .get_public_sub_msg(
                &WsChannel::Lob(Some(LobParam::Incremental {
                    depth: Some(50),
                    frequency: Some(LobFrequency::Ms20),
                })),
                Some(&insts),
            )
            .await;
        assert!(err.is_err());
    }
}
