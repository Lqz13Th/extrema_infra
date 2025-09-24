use std::{
    sync::Arc,
    collections::HashMap,
};
use simd_json::from_slice;
use serde_json::json;
use reqwest::Client;
use tracing::{error, warn};

use crate::errors::{InfraError, InfraResult};
use crate::market_assets::{
    api_general::*,
    base_data::*,
    account_data::*,
    price_data::*,
    utils_data::*,
};
use crate::task_execution::task_ws::*;
use crate::traits::{
    market_cex::{CexWebsocket, CexPrivateRest, CexPublicRest, MarketCexApi}
};

use super::{
    api_key::{OkxKey, read_okx_env_key},
    api_utils::*,
    config_assets::*,
    rest::{
        account_balance::RestAccountBalOkx,
        account_positions::RestAccountPosOkx,
        ct_current_lead_traders::RestLeadtraderOkx,
        ct_public_lead_traders::RestPubLeadTradersOkx,
        ct_public_current_subpositions::RestSubPositionOkx,
        ct_public_lead_trader_stats::RestPubLeadTraderStatsOkx,
        ct_public_subpositions_history::RestSubPositionHistoryOkx,
        market_ticker::RestMarketTickerOkx,
        public_instruments::RestInstrumentsOkx,
        trade_order::RestOrderAckOkx,
    }
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


impl MarketCexApi for OkxCli {}

impl CexPublicRest for OkxCli {
    async fn get_ticker(
        &self,
        insts: &str,
    ) -> InfraResult<TickerData> {
        self._get_ticker(insts).await
    }

    async fn get_instrument_info(
        &self,
        inst_type: InstrumentType
    ) -> InfraResult<Vec<InstrumentInfo>> {
        self._get_instrument_info(inst_type).await
    }
}

impl CexPrivateRest for OkxCli {
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

    async fn place_order(
        &self,
        order_params: OrderParams,
    ) -> InfraResult<OrderAckData> {
        self._place_order(order_params).await
    }

    async fn get_balance(
        &self,
        assets: Option<&[String]>,
    ) -> InfraResult<Vec<BalanceData>> {
        self._get_balance(assets).await
    }

    async fn get_position(
        &self,
        insts: Option<&[String]>,
    ) -> InfraResult<Vec<PositionData>> {
        self._get_positions(insts).await
    }
}



impl CexWebsocket for OkxCli {
    async fn get_public_sub_msg(
        &self,
        channel: &WsChannel,
        insts: Option<&[String]>
    ) -> InfraResult<String> {
        self._get_public_sub_msg(channel, insts)
    }

    async fn get_private_sub_msg(
        &self,
        channel: &WsChannel
    ) -> InfraResult<String> {
        let args = match channel {
            WsChannel::AccountOrder => {
                vec![json!({
                    "channel": "orders",
                    "instType": "ANY",
                })]
            },
            WsChannel::AccountPosition => {
                vec![json!({
                    "channel": "positions",
                    "instType": "ANY",
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

    async fn get_public_connect_msg(
        &self,
        channel: &WsChannel,
    ) -> InfraResult<String> {
        let url = match channel {
            WsChannel::Candle(_)
            | WsChannel::Trades(_)
            | WsChannel::Tick
            | WsChannel::Lob => OKX_WS_PUB,

            WsChannel::Other(s) if s == "instruments"
                || s == "funding-rate" => OKX_WS_BUS,

            _ => return Err(InfraError::Unimplemented),
        };

        Ok(url.into())
    }

    async fn get_private_connect_msg(
        &self,
        _channel: &WsChannel
    ) -> InfraResult<String> {
        Ok(OKX_WS_PRI.into())
    }
}

impl OkxCli {
    pub fn new(shared_client: Arc<Client>) -> Self {
        Self {
            client: shared_client,
            api_key: None
        }
    }

    pub fn ws_login_msg(&self) -> InfraResult<String> {
        let api_key = self.api_key.as_ref().ok_or(InfraError::ApiNotInitialized)?;

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

    pub async fn get_current_lead_traders(
        &self,
        inst_type: Option<InstrumentType>,
    ) -> InfraResult<Vec<CurrentLeadtrader>> {
        let inst_type_str = match inst_type.unwrap_or(InstrumentType::Perpetual) {
            InstrumentType::Spot => "SPOT",
            InstrumentType::Futures => "FUTURES",
            InstrumentType::Perpetual => "SWAP",
            InstrumentType::Option => "OPTION",
            InstrumentType::Unknown => {
                return Err(InfraError::ApiError("Unknown instrument type".into()))
            },
        };

        let body = json!({
            "instType": inst_type_str,
        }).to_string();

        let res: RestResOkx<RestLeadtraderOkx> = self.api_key
            .as_ref()
            .ok_or(InfraError::ApiNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                body,
                OKX_BASE_URL,
                OKX_CT_CURRENT_LEADTRADERS,
            )
            .await?;

        if res.code != "0" {
            let msg = res.msg.unwrap_or_default();
            return Err(InfraError::ApiError(format!("code {}: {}", res.code, msg)));
        }

        let data: Vec<CurrentLeadtrader> = res.data
            .unwrap_or_default()
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
            InstrumentType::Option => "OPTION",
            InstrumentType::Unknown => {
                return Err(InfraError::ApiError("Unknown instrument type".into()))
            },
        };

        let mut url = format!(
            "{}{}?instType={}",
            OKX_BASE_URL,
            OKX_CT_PUBLIC_LEADTRADERS,
            inst_type_str,
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

        let responds = self.client.get(&url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: RestResOkx<RestPubLeadTradersOkx> = from_slice(&mut res_bytes)?;

        if res.code != "0" {
            let msg = res.msg.unwrap_or_default();
            return Err(InfraError::ApiError(format!("code {}: {}", res.code, msg)));
        };

        let data = res
            .data
            .unwrap_or_default()
            .into_iter()
            .next()
            .ok_or(InfraError::ApiError("No data returned".into()))?;

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
            InstrumentType::Option => "OPTION",
            InstrumentType::Unknown => {
                return Err(InfraError::ApiError("Unknown instrument type".into()))
            },
        };

        let url = format!(
            "{}{}?uniqueCode={}&instType={}&lastDays={}",
            OKX_BASE_URL,
            OKX_CT_PUBLIC_LEADTRADER_STATS,
            unique_code,
            inst_type_str,
            last_days,
        );


        let responds = self.client.get(&url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();

        let res: RestResOkx<RestPubLeadTraderStatsOkx> = from_slice(&mut res_bytes)?;

        if res.code != "0" {
            let msg = res.msg.unwrap_or_default();
            return Err(InfraError::ApiError(format!(
                "code {}: {}",
                res.code, msg
            )));
        }

        let data: Vec<PubLeadtraderStats> = res.data
            .unwrap_or_default()
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
            InstrumentType::Option => "OPTION",
            InstrumentType::Unknown => {
                return Err(InfraError::ApiError("Unknown instrument type".into()))
            },
        };

        let mut url = format!(
            "{}{}?uniqueCode={}&instType={}",
            OKX_BASE_URL,
            OKX_CT_LEADTRADER_SUBPOSITIONS,
            unique_code,
            inst_type_str,
        );

        if let Some(l) = limit {
            url.push_str(&format!("&limit={}", l));
        }

        let responds = self.client.get(&url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: RestResOkx<RestSubPositionOkx> = from_slice(&mut res_bytes)?;

        if res.code != "0" {
            let msg = res.msg.unwrap_or_default();
            return Err(InfraError::ApiError(format!("code {}: {}", res.code, msg)));
        }

        let data: Vec<LeadtraderSubposition> = res.data
            .unwrap_or_default()
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
            InstrumentType::Option => "OPTION",
            InstrumentType::Unknown => {
                return Err(InfraError::ApiError("Unknown instrument type".into()))
            },
        };

        let mut url = format!(
            "{}{}?uniqueCode={}&instType={}",
            OKX_BASE_URL,
            OKX_CT_LEADTRADER_SUBPOSITIONS_HISTORY,
            unique_code,
            inst_type_str,
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

        let responds = self.client.get(&url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: RestResOkx<RestSubPositionHistoryOkx> = from_slice(&mut res_bytes)?;

        if res.code != "0" {
            let msg = res.msg.unwrap_or_default();
            return Err(InfraError::ApiError(format!("code {}: {}", res.code, msg)));
        }

        let data: Vec<LeadtraderSubpositionHistory> = res.data
            .unwrap_or_default()
            .into_iter()
            .map(LeadtraderSubpositionHistory::from)
            .collect();

        Ok(data)
    }

    async fn _get_ticker(
        &self,
        inst: &str,
    ) -> InfraResult<TickerData> {
        let url = format!(
            "{}{}?&instId={}",
            OKX_BASE_URL,
            OKX_MARKET_TICKER,
            to_okx_inst(inst),
        );

        let responds = self.client.get(&url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: RestResOkx<RestMarketTickerOkx> = from_slice(&mut res_bytes)?;

        if res.code != "0" {
            let msg = res.msg.unwrap_or_default();
            return Err(InfraError::ApiError(format!("code {}: {}", res.code, msg)));
        }

        let data: TickerData = res.data
            .unwrap_or_default()
            .into_iter()
            .next()
            .map(TickerData::from)
            .ok_or_else(|| InfraError::ApiError("Empty market tick data".into()))?;

        Ok(data)
    }

    async fn _get_instrument_info(
        &self,
        inst_type: InstrumentType
    ) -> InfraResult<Vec<InstrumentInfo>> {
        let inst_type_str = match inst_type {
            InstrumentType::Spot => "SPOT",
            InstrumentType::Futures => "FUTURES",
            InstrumentType::Perpetual => "SWAP",
            InstrumentType::Option => "OPTION",
            InstrumentType::Unknown => {
                return Err(InfraError::ApiError("Unknown instrument type".into()))
            },
        };

        let url = format!(
            "{}{}?&instType={}",
            OKX_BASE_URL,
            OKX_PUBLIC_INSTRUMENTS,
            inst_type_str,
        );

        let responds = self.client.get(&url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: RestResOkx<RestInstrumentsOkx> = from_slice(&mut res_bytes)?;

        if res.code != "0" {
            let msg = res.msg.unwrap_or_default();
            return Err(InfraError::ApiError(format!("code {}: {}", res.code, msg)));
        }

        let data: Vec<InstrumentInfo> = res.data
            .unwrap_or_default()
            .into_iter()
            .map(InstrumentInfo::from)
            .collect();

        Ok(data)
    }

    async fn _place_order(
        &self,
        order_params: OrderParams,
    ) -> InfraResult<OrderAckData> {
        let mut body = json!({
            "instId": to_okx_inst(&order_params.inst),
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
                PositionSide::Unknown => "net",
            });
        }

        if let Some(cl_id) = order_params.client_order_id {
            body["clOrdId"] = json!(cl_id);
        }

        for (k, v) in order_params.extra {
            body[k] = json!(v);
        }

        let res: RestResOkx<RestOrderAckOkx> = self.api_key
            .as_ref()
            .ok_or(InfraError::ApiNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Post,
                body.to_string(),
                OKX_BASE_URL,
                OKX_TRADE_ORDER,
            )
            .await?;

        warn!("{:?}", res);
        if res.code != "0" {
            return Err(InfraError::ApiError(res.msg.unwrap_or_default()));
        }


        let data: OrderAckData = res
            .data
            .unwrap_or_default()
            .into_iter()
            .map(OrderAckData::from)
            .next()
            .ok_or_else(|| InfraError::ApiError("Empty order ack data".into()))?;

        Ok(data)
    }


    async fn _get_balance(
        &self,
        assets: Option<&[String]>,
    ) -> InfraResult<Vec<BalanceData>> {
        let body = match assets {
            Some(ccys) if !ccys.is_empty() => {
                let okx_ccys: Vec<String> = ccys.iter().map(|s| to_okx_inst(s)).collect();
                json!({ "ccy": okx_ccys.join(",") }).to_string()
            }
            _ => "{}".into(),
        };

        let bal_res: RestResOkx<RestAccountBalOkx> = self.api_key
            .as_ref()
            .ok_or(InfraError::ApiNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                body,
                OKX_BASE_URL,
                OKX_ACCOUNT_BALANCE,
            )
            .await?;

        if bal_res.code != "0" {
            let msg = bal_res.msg.unwrap_or_default();
            return Err(InfraError::ApiError(format!("code {}: {}", bal_res.code, msg)));
        }

        let data: Vec<BalanceData> = bal_res
            .data
            .unwrap_or_default()
            .into_iter()
            .flat_map(|account| account.details)
            .map(BalanceData::from)
            .collect();

        Ok(data)
    }

    async fn _get_positions(
        &self,
        insts: Option<&[String]>,
    ) -> InfraResult<Vec<PositionData>> {
        let body = match insts {
            Some(ids) if !ids.is_empty() => {
                let okx_ids: Vec<String> = ids.iter().map(|s| to_okx_inst(s)).collect();
                json!({ "instId": okx_ids.join(",") }).to_string()
            },
            _ => "{}".into(),
        };

        let pos_res: RestResOkx<RestAccountPosOkx> = self.api_key
            .as_ref()
            .ok_or(InfraError::ApiNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                body,
                OKX_BASE_URL,
                OKX_ACCOUNT_POSITIONS,
            )
            .await?;

        if pos_res.code != "0" {
            let msg = pos_res.msg.unwrap_or_default();
            return Err(InfraError::ApiError(format!("code {}: {}", pos_res.code, msg)));
        }

        let data: Vec<PositionData> = pos_res
            .data
            .unwrap_or_default()
            .into_iter()
            .map(PositionData::from) // 你可以像 BalanceData 一样实现 From
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

        let channel = format!("candle{}", interval);

        Ok(ws_subscribe_msg_okx(&channel, insts))
    }
}

