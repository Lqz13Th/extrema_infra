use reqwest::Client;
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use tracing::error;

use crate::arch::{
    market_assets::{
        api_data::{account_data::*, price_data::*, utils_data::*},
        api_general::*,
        base_data::*,
    },
    task_execution::task_ws::*,
    traits::{
        conversion::IntoInfraVec,
        market_lob::{LobPrivateRest, LobPublicRest, LobWebsocket, MarketLobApi},
    },
};
use crate::errors::{InfraError, InfraResult};

use super::{
    api_key::{OkxKey, read_okx_env_key},
    api_utils::*,
    config_assets::*,
    okx_rest_msg::RestResOkx,
    schemas::rest::{
        account_balance::RestAccountBalOkx, account_positions::RestAccountPosOkx,
        account_set_leverage::RestAccountSetLeverageOkx,
        ct_current_lead_traders::RestLeadtraderOkx,
        ct_public_current_subpositions::RestSubPositionOkx,
        ct_public_lead_trader_stats::RestPubLeadTraderStatsOkx,
        ct_public_lead_traders::RestPubLeadTradersOkx,
        ct_public_subpositions_history::RestSubPositionHistoryOkx,
        funding_rate::RestFundingRateOkx, funding_rate_history::RestFundingRateHistoryOkx,
        market_ticker::RestMarketTickerOkx, order_history::RestOrderHistoryOkx,
        price_limit::RestPriceLimitOkx, public_instruments::RestInstrumentsOkx,
        trade_order::RestOrderAckOkx,
    },
};

fn create_okx_cli_with_key(
    keys: HashMap<String, OkxKey>,
    shared_client: Arc<Client>,
) -> HashMap<String, OkxCli> {
    keys.into_iter()
        .map(|(id, key)| {
            let cli = OkxCli {
                client: shared_client.clone(),
                api_key: Some(key),
            };
            (id, cli)
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct OkxCli {
    pub client: Arc<Client>,
    pub api_key: Option<OkxKey>,
}

impl Default for OkxCli {
    fn default() -> Self {
        Self::new(Arc::new(Client::new()))
    }
}

impl MarketLobApi for OkxCli {}

impl LobPublicRest for OkxCli {
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
}

impl LobPrivateRest for OkxCli {
    fn init_api_key(&mut self) {
        match read_okx_env_key() {
            Ok(okx_key) => {
                self.api_key = Some(okx_key);
            },
            Err(e) => {
                error!("Failed to read OKX env key: {:?}", e);
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

impl LobWebsocket for OkxCli {
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

    async fn get_public_connect_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        self._get_public_connect_msg(channel)
    }

    async fn get_private_connect_msg(&self, _channel: &WsChannel) -> InfraResult<String> {
        Ok(OKX_WS_PRI.into())
    }
}

impl OkxCli {
    pub fn new(shared_client: Arc<Client>) -> Self {
        Self {
            client: shared_client,
            api_key: None,
        }
    }

    pub fn ws_login_msg(&self) -> InfraResult<String> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?;

        let timestamp = get_okx_timestamp();
        let raw_sign = format!("{}{}", timestamp, OKX_WS_LOGIN);
        let signature = api_key.sign(raw_sign, timestamp.clone())?;

        let msg = json!({
            "op": "login",
            "args": [{
                "apiKey": api_key.api_key,
                "passphrase": api_key.passphrase,
                "timestamp": timestamp,
                "sign": signature.signature
            }]
        });

        Ok(msg.to_string())
    }

    pub async fn set_leverage(
        &self,
        inst: &str,
        leverage: u32,
        margin_mode: MarginMode,
    ) -> InfraResult<RestAccountSetLeverageOkx> {
        if leverage == 0 {
            return Err(InfraError::ApiCliError(
                "OKX leverage must be greater than 0".into(),
            ));
        }

        let mgn_mode = match margin_mode {
            MarginMode::Cross => "cross",
            MarginMode::Isolated => "isolated",
            MarginMode::Unknown => {
                return Err(InfraError::ApiCliError(
                    "Unknown margin mode for OKX set_leverage".into(),
                ));
            },
        };

        let body = json!({
            "instId": cli_perp_to_okx_inst(inst),
            "lever": leverage.to_string(),
            "mgnMode": mgn_mode,
        })
        .to_string();

        let res: RestResOkx<RestAccountSetLeverageOkx> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Post,
                body,
                OKX_BASE_URL,
                OKX_ACCOUNT_SET_LEVERAGE,
            )
            .await?;

        let data = res
            .into_vec()?
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError(
                "No OKX leverage data returned".into(),
            ))?;

        Ok(data)
    }

    pub async fn get_current_lead_traders(
        &self,
        inst_type: Option<InstrumentType>,
    ) -> InfraResult<Vec<CurrentLeadtrader>> {
        let inst_type_str = match inst_type.unwrap_or(InstrumentType::Perpetual) {
            InstrumentType::Spot => "SPOT",
            InstrumentType::Futures => "FUTURES",
            InstrumentType::Perpetual => "SWAP",
            InstrumentType::Options => "OPTION",
            InstrumentType::Unknown => {
                return Err(InfraError::ApiCliError("Unknown instrument type".into()));
            },
        };

        let body = json!({
            "instType": inst_type_str,
        })
        .to_string();

        let res: RestResOkx<RestLeadtraderOkx> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                body,
                OKX_BASE_URL,
                OKX_CT_CURRENT_LEADTRADERS,
            )
            .await?;

        let data: Vec<CurrentLeadtrader> = res
            .into_vec()?
            .into_iter()
            .map(CurrentLeadtrader::from)
            .collect();

        Ok(data)
    }

    pub async fn get_public_lead_traders(
        &self,
        query: PubLeadTraderQuery,
    ) -> InfraResult<PubLeadtraderInfo> {
        let inst_type_str = match query.inst_type.unwrap_or(InstrumentType::Perpetual) {
            InstrumentType::Spot => "SPOT",
            InstrumentType::Futures => "FUTURES",
            InstrumentType::Perpetual => "SWAP",
            InstrumentType::Options => "OPTION",
            InstrumentType::Unknown => {
                return Err(InfraError::ApiCliError("Unknown instrument type".into()));
            },
        };

        let mut url = format!(
            "{}{}?instType={}",
            OKX_BASE_URL, OKX_CT_PUBLIC_LEADTRADERS, inst_type_str,
        );

        if let Some(sort) = query.sort_type {
            url.push_str(&format!("&sortType={}", sort));
        }
        if let Some(state) = query.state {
            url.push_str(&format!("&state={}", state));
        }
        if let Some(days) = query.min_lead_days {
            url.push_str(&format!("&minLeadDays={}", days));
        }
        if let Some(min_assets) = query.min_assets {
            url.push_str(&format!("&minAssets={}", min_assets));
        }
        if let Some(max_assets) = query.max_assets {
            url.push_str(&format!("&maxAssets={}", max_assets));
        }
        if let Some(min_aum) = query.min_aum {
            url.push_str(&format!("&minAum={}", min_aum));
        }
        if let Some(max_aum) = query.max_aum {
            url.push_str(&format!("&maxAum={}", max_aum));
        }
        if let Some(data_ver) = query.data_ver {
            url.push_str(&format!("&dataVer={}", data_ver));
        }
        if let Some(page) = query.page {
            url.push_str(&format!("&page={}", page));
        }
        if let Some(limit) = query.limit {
            url.push_str(&format!("&limit={}", limit));
        }

        let response = self.client.get(url).send().await?;
        let res: RestResOkx<RestPubLeadTradersOkx> =
            parse_json_response("Okx public_lead_traders", response).await?;

        let data = res
            .into_vec()?
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError(
                "No public lead traders data returned".into(),
            ))?;

        Ok(PubLeadtraderInfo::from(data))
    }

    pub async fn get_public_lead_trader_stats(
        &self,
        unique_code: &str,
        last_days: u64,
        inst_type: Option<InstrumentType>,
    ) -> InfraResult<Vec<PubLeadtraderStats>> {
        let inst_type_str = match inst_type.unwrap_or(InstrumentType::Perpetual) {
            InstrumentType::Spot => "SPOT",
            InstrumentType::Futures => "FUTURES",
            InstrumentType::Perpetual => "SWAP",
            InstrumentType::Options => "OPTION",
            InstrumentType::Unknown => {
                return Err(InfraError::ApiCliError("Unknown instrument type".into()));
            },
        };

        let url = format!(
            "{}{}?uniqueCode={}&instType={}&lastDays={}",
            OKX_BASE_URL, OKX_CT_PUBLIC_LEADTRADER_STATS, unique_code, inst_type_str, last_days,
        );

        let response = self.client.get(url).send().await?;
        let res: RestResOkx<RestPubLeadTraderStatsOkx> =
            parse_json_response("Okx public_lead_trader_stats", response).await?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(PubLeadtraderStats::from)
            .collect();

        Ok(data)
    }

    pub async fn get_lead_trader_subpositions(
        &self,
        unique_code: &str,
        inst_type: Option<InstrumentType>,
        limit: Option<u32>,
    ) -> InfraResult<Vec<LeadtraderSubposition>> {
        let inst_type_str = match inst_type.unwrap_or(InstrumentType::Perpetual) {
            InstrumentType::Spot => "SPOT",
            InstrumentType::Futures => "FUTURES",
            InstrumentType::Perpetual => "SWAP",
            InstrumentType::Options => "OPTION",
            InstrumentType::Unknown => {
                return Err(InfraError::ApiCliError("Unknown instrument type".into()));
            },
        };

        let mut url = format!(
            "{}{}?uniqueCode={}&instType={}",
            OKX_BASE_URL, OKX_CT_LEADTRADER_SUBPOSITIONS, unique_code, inst_type_str,
        );

        if let Some(l) = limit {
            url.push_str(&format!("&limit={}", l));
        }

        let response = self.client.get(url).send().await?;
        let res: RestResOkx<RestSubPositionOkx> =
            parse_json_response("Okx lead_trader_subpositions", response).await?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(LeadtraderSubposition::from)
            .collect();

        Ok(data)
    }

    pub async fn get_lead_trader_subpositions_history(
        &self,
        unique_code: &str,
        inst_type: Option<InstrumentType>,
        limit: Option<u32>,
        before: Option<&str>,
        after: Option<&str>,
    ) -> InfraResult<Vec<LeadtraderSubpositionHistory>> {
        let inst_type_str = match inst_type.unwrap_or(InstrumentType::Perpetual) {
            InstrumentType::Spot => "SPOT",
            InstrumentType::Futures => "FUTURES",
            InstrumentType::Perpetual => "SWAP",
            InstrumentType::Options => "OPTION",
            InstrumentType::Unknown => {
                return Err(InfraError::ApiCliError("Unknown instrument type".into()));
            },
        };

        let mut url = format!(
            "{}{}?uniqueCode={}&instType={}",
            OKX_BASE_URL, OKX_CT_LEADTRADER_SUBPOSITIONS_HISTORY, unique_code, inst_type_str,
        );

        if let Some(l) = limit {
            url.push_str(&format!("&limit={}", l));
        }
        if let Some(b) = before {
            url.push_str(&format!("&before={}", b));
        }
        if let Some(a) = after {
            url.push_str(&format!("&after={}", a));
        }

        let response = self.client.get(url).send().await?;
        let res: RestResOkx<RestSubPositionHistoryOkx> =
            parse_json_response("Okx lead_trader_subpositions_history", response).await?;

        let data: Vec<LeadtraderSubpositionHistory> = res
            .into_vec()?
            .into_iter()
            .map(LeadtraderSubpositionHistory::from)
            .collect();

        Ok(data)
    }

    pub async fn get_funding_rate_info(
        &self,
        inst: Option<&str>,
    ) -> InfraResult<Vec<FundingRateInfo>> {
        let inst_id = inst
            .map(cli_perp_to_okx_inst)
            .unwrap_or_else(|| "ANY".to_string());

        let url = format!(
            "{}{}?instId={}",
            OKX_BASE_URL, OKX_PUBLIC_FUNDING_RATE, inst_id
        );

        let response = self.client.get(url).send().await?;
        let res: RestResOkx<RestFundingRateOkx> =
            parse_json_response("Okx funding_rate_info", response).await?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(FundingRateInfo::from)
            .collect();

        Ok(data)
    }

    pub async fn get_funding_rate_live(
        &self,
        inst: Option<&str>,
    ) -> InfraResult<Vec<FundingRateData>> {
        let inst_id = inst
            .map(cli_perp_to_okx_inst)
            .unwrap_or_else(|| "ANY".to_string());

        let url = format!(
            "{}{}?instId={}",
            OKX_BASE_URL, OKX_PUBLIC_FUNDING_RATE, inst_id
        );

        let response = self.client.get(url).send().await?;
        let res: RestResOkx<RestFundingRateOkx> =
            parse_json_response("Okx funding_rate_live", response).await?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(FundingRateData::from)
            .collect();

        Ok(data)
    }

    pub async fn get_funding_rate_history(
        &self,
        inst: &str,
        limit: Option<u32>,
        before_ms: Option<u64>,
        after_ms: Option<u64>,
    ) -> InfraResult<Vec<FundingRateData>> {
        let mut params = vec![format!("instId={}", cli_perp_to_okx_inst(inst))];
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }
        if let Some(b) = before_ms {
            params.push(format!("before={}", b));
        }
        if let Some(a) = after_ms {
            params.push(format!("after={}", a));
        }

        let url = format!(
            "{}{}?{}",
            OKX_BASE_URL,
            OKX_PUBLIC_FUNDING_RATE_HISTORY,
            params.join("&")
        );

        let response = self.client.get(url).send().await?;
        let res: RestResOkx<RestFundingRateHistoryOkx> =
            parse_json_response("Okx funding_rate_history", response).await?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(FundingRateData::from)
            .collect();

        Ok(data)
    }

    pub async fn get_price_limit(&self, inst: &str) -> InfraResult<Vec<RestPriceLimitOkx>> {
        let url = format!(
            "{}{}?instId={}",
            OKX_BASE_URL,
            OKX_PUBLIC_PRICE_LIMIT,
            cli_perp_to_okx_inst(inst)
        );

        let response = self.client.get(url).send().await?;
        let res: RestResOkx<RestPriceLimitOkx> =
            parse_json_response("Okx price_limit", response).await?;

        res.into_vec()
    }

    async fn _get_tickers(
        &self,
        insts: Option<&[String]>,
        inst_type: Option<InstrumentType>,
    ) -> InfraResult<Vec<TickerData>> {
        let inst_type_str = match inst_type.unwrap_or(InstrumentType::Perpetual) {
            InstrumentType::Spot => "SPOT",
            InstrumentType::Futures => "FUTURES",
            InstrumentType::Perpetual => "SWAP",
            InstrumentType::Options => "OPTION",
            InstrumentType::Unknown => {
                return Err(InfraError::ApiCliError("Unknown instrument type".into()));
            },
        };

        let url = format!(
            "{}{}?instType={}",
            OKX_BASE_URL, OKX_MARKET_TICKERS, inst_type_str
        );
        let response = self.client.get(url).send().await?;
        let res: RestResOkx<RestMarketTickerOkx> =
            parse_json_response("Okx tickers", response).await?;

        let data = res
            .into_vec()?
            .into_iter()
            .filter(|t| match insts {
                Some(list) => list.contains(&okx_inst_to_cli(&t.instId)), // BTC-USDT
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
        let inst_type_str = match inst_type {
            InstrumentType::Spot => "SPOT",
            InstrumentType::Futures => "FUTURES",
            InstrumentType::Perpetual => "SWAP",
            InstrumentType::Options => "OPTION",
            InstrumentType::Unknown => {
                return Err(InfraError::ApiCliError("Unknown instrument type".into()));
            },
        };

        let url = format!(
            "{}{}?&instType={}",
            OKX_BASE_URL, OKX_PUBLIC_INSTRUMENTS, inst_type_str,
        );

        let response = self.client.get(url).send().await?;
        let res: RestResOkx<RestInstrumentsOkx> =
            parse_json_response("Okx instrument_info", response).await?;

        let data: Vec<InstrumentInfo> = res
            .into_vec()?
            .into_iter()
            .map(InstrumentInfo::from)
            .collect();

        Ok(data)
    }

    async fn _place_order(&self, order_params: OrderParams) -> InfraResult<OrderAckData> {
        let mut body = json!({
            "instId": cli_perp_to_okx_inst(&order_params.inst),
            "side": match order_params.side {
                OrderSide::BUY => "buy",
                OrderSide::SELL => "sell",
                _ => "buy", // fallback
            },
            "sz": order_params.size,
            "ordType": match order_params.order_type {
                OrderType::Limit => "limit",
                OrderType::Market => "market",
                OrderType::PostOnly => "post_only",
                OrderType::Fok => "fok",
                OrderType::Ioc => "ioc",
                OrderType::Unknown => "market",
            },
        });

        if let Some(price) = order_params.price {
            body["px"] = json!(price);
        }

        if let Some(reduce_only) = order_params.reduce_only {
            body["reduceOnly"] = json!(reduce_only);
        }

        if let Some(td_mode) = order_params.margin_mode {
            body["tdMode"] = json!(match td_mode {
                MarginMode::Isolated => "isolated",
                MarginMode::Cross => "cross",
                MarginMode::Unknown => "isolated",
            });
        }

        if let Some(pos_side) = order_params.position_side {
            body["posSide"] = json!(match pos_side {
                PositionSide::Long => "long",
                PositionSide::Short => "short",
                PositionSide::Both => "net",
                PositionSide::Unknown => "net",
            });
        }

        if let Some(cl_id) = order_params.client_order_id {
            body["clOrdId"] = json!(cl_id);
        }

        for (k, v) in order_params.extra {
            body[k] = json!(v);
        }

        let res: RestResOkx<RestOrderAckOkx> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Post,
                body.to_string(),
                OKX_BASE_URL,
                OKX_TRADE_ORDER,
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
        let body = match assets {
            Some(ccys) if !ccys.is_empty() => {
                let okx_ccys: Vec<String> = ccys.iter().map(|s| cli_perp_to_okx_inst(s)).collect();
                json!({ "ccy": okx_ccys.join(",") }).to_string()
            },
            _ => "{}".into(),
        };

        let res: RestResOkx<RestAccountBalOkx> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                body,
                OKX_BASE_URL,
                OKX_ACCOUNT_BALANCE,
            )
            .await?;

        let data: Vec<BalanceData> = res
            .into_vec()?
            .into_iter()
            .flat_map(|account| account.details)
            .map(BalanceData::from)
            .collect();

        Ok(data)
    }

    async fn _get_positions(&self, insts: Option<&[String]>) -> InfraResult<Vec<PositionData>> {
        let body = match insts {
            Some(ids) if !ids.is_empty() => {
                let okx_ids: Vec<String> = ids.iter().map(|s| cli_perp_to_okx_inst(s)).collect();
                json!({ "instId": okx_ids.join(",") }).to_string()
            },
            _ => "{}".into(),
        };

        let res: RestResOkx<RestAccountPosOkx> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                body,
                OKX_BASE_URL,
                OKX_ACCOUNT_POSITIONS,
            )
            .await?;

        let data: Vec<PositionData> = res
            .into_vec()?
            .into_iter()
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
        let okx_inst = cli_perp_to_okx_inst(inst);
        let mut query = format!("instId={}", okx_inst);
        let endpoint = if let Some(order_id) = order_id {
            query.push_str(&format!("&ordId={}", order_id));
            OKX_TRADE_ORDER
        } else {
            query.push_str("&instType=SWAP");
            if let Some(start_time) = start_time {
                query.push_str(&format!("&begin={}", start_time));
            }
            if let Some(end_time) = end_time {
                query.push_str(&format!("&end={}", end_time));
            }
            if let Some(limit) = limit {
                query.push_str(&format!("&limit={}", limit));
            }
            OKX_TRADE_ORDERS_HISTORY
        };

        let res: RestResOkx<RestOrderHistoryOkx> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                query,
                OKX_BASE_URL,
                endpoint,
            )
            .await?;

        let data: Vec<HistoOrderData> = res
            .into_vec()?
            .into_iter()
            .map(HistoOrderData::from)
            .collect();

        Ok(data)
    }

    fn _get_public_connect_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        let url = match channel {
            WsChannel::Trades(Some(trades_param)) => match trades_param {
                TradesParam::AggTrades => OKX_WS_PUB,
                TradesParam::AllTrades => OKX_WS_BUS,
            },
            WsChannel::Candles(_) | WsChannel::Tick | WsChannel::Lob | WsChannel::Trades(None) => {
                OKX_WS_PUB
            },
            WsChannel::Other(s) if s == "instruments" || s == "funding-rate" => OKX_WS_BUS,
            _ => return Err(InfraError::Unimplemented),
        };

        Ok(url.into())
    }

    fn _get_public_sub_msg(
        &self,
        ws_channel: &WsChannel,
        insts: Option<&[String]>,
    ) -> InfraResult<String> {
        match ws_channel {
            WsChannel::Candles(channel) => self._ws_subscribe_candle(channel, insts),
            WsChannel::Trades(trades_param) => self._ws_subscribe_trades(trades_param, insts),
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

        let channel = format!("candle{}", interval);

        Ok(ws_subscribe_msg_okx(&channel, insts))
    }

    fn _ws_subscribe_trades(
        &self,
        trades_param: &Option<TradesParam>,
        insts: Option<&[String]>,
    ) -> InfraResult<String> {
        let channel = match trades_param {
            Some(TradesParam::AggTrades) | None => "trades",
            Some(TradesParam::AllTrades) => "tradesAll",
        };

        Ok(ws_subscribe_msg_okx(channel, insts))
    }

    fn _get_private_sub_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        let args = match channel {
            WsChannel::AccountOrders => {
                vec![json!({
                    "channel": "orders",
                    "instType": "ANY",
                })]
            },
            WsChannel::AccountPositions => {
                vec![json!({
                    "channel": "positions",
                    "instType": "ANY",
                })]
            },
            WsChannel::AccountBalAndPos => {
                vec![json!({
                    "channel": "balance_and_position",
                })]
            },
            _ => return Err(InfraError::Unimplemented),
        };

        let msg = json!({
            "op": "subscribe",
            "args": args
        });

        Ok(msg.to_string())
    }
}
