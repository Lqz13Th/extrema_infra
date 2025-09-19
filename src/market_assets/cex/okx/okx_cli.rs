use std::{
    sync::Arc,
    collections::HashMap,
};
use serde_json::{from_str, json};
use reqwest::Client;
use tracing::error;

use crate::errors::{InfraError, InfraResult};
use crate::market_assets::{
    api_general::RequestMethod,
    base_data::InstrumentType,
    account_data::*,
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
        current_lead_traders::RestLeadtraderOkx,
        public_lead_traders::RestPubLeadTradersOkx,
        public_current_subpositions::RestSubPositionOkx,
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

    async fn get_balance(
        &self,
        assets: Vec<String>,
    ) -> InfraResult<Vec<BalanceData>> {
        self._get_balance(assets).await
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
                })]            },
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

        Ok(url.to_string())
    }

    async fn get_private_connect_msg(
        &self,
        _channel: &WsChannel
    ) -> InfraResult<String> {
        Ok(OKX_WS_PRI.to_string())
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
            InstrumentType::Perpetual => "SWAP",
            InstrumentType::Option => "OPTION",
            InstrumentType::Unknown => "SPOT",
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
                OKX_CURRENT_LEADTRADERS,
            )
            .await?;

        if res.code != "0" {
            let msg = res.msg.unwrap_or_default();
            return Err(InfraError::ApiError(format!("code {}: {}", res.code, msg)));
        }

        let result: Vec<CurrentLeadtrader> = res
            .data
            .into_iter()
            .map(CurrentLeadtrader::from)
            .collect();

        Ok(result)
    }

    pub async fn get_public_lead_traders(
        &self,
        query: PubLeadTraderQuery,
    ) -> InfraResult<PubLeadtraderInfo> {
        let inst_type_str = match query.inst_type.unwrap_or(InstrumentType::Perpetual) {
            InstrumentType::Spot => "SPOT",
            InstrumentType::Perpetual => "SWAP",
            InstrumentType::Option => "OPTION",
            InstrumentType::Unknown => "SPOT",
        };

        let mut url = format!("{}{}?instType={}", OKX_BASE_URL, OKX_PUBLIC_LEADTRADERS, inst_type_str);

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

        let text = self.client
            .get(&url)
            .send().await?
            .text().await?;

        let res: RestResOkx<RestPubLeadTradersOkx> = from_str(&text)?;

        if res.code != "0" {
            let msg = res.msg.unwrap_or_default();
            return Err(InfraError::ApiError(format!("code {}: {}", res.code, msg)));
        };

        let first_data = res
            .data
            .into_iter()
            .next()
            .ok_or(InfraError::ApiError("No data returned".into()))?;

        Ok(PubLeadtraderInfo::from(first_data))
    }

    pub async fn get_lead_trader_subpositions(
        &self,
        unique_code: &str,
        inst_type: Option<InstrumentType>,
        limit: Option<u32>,
        before: Option<&str>,
        after: Option<&str>,
    ) -> InfraResult<Vec<LeadtraderSubpositions>> {
        let inst_type_str = match inst_type.unwrap_or(InstrumentType::Perpetual) {
            InstrumentType::Spot => "SPOT",
            InstrumentType::Perpetual => "SWAP",
            InstrumentType::Option => "OPTION",
            InstrumentType::Unknown => "SPOT",
        };

        let mut url = format!(
            "{}{}?uniqueCode={}&instType={}",
            OKX_BASE_URL,
            OKX_LEADTRADER_SUBPOSITIONS,
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

        let text = self.client
            .get(&url)
            .send().await?
            .text().await?;

        let res: RestResOkx<RestSubPositionOkx> = from_str(&text)?;

        if res.code != "0" {
            let msg = res.msg.unwrap_or_default();
            return Err(InfraError::ApiError(format!("code {}: {}", res.code, msg)));
        }

        let result: Vec<LeadtraderSubpositions> = res
            .data
            .into_iter()
            .map(LeadtraderSubpositions::from)
            .collect();

        Ok(result)
    }

    async fn _get_balance(
        &self,
        assets: Vec<String>,
    ) -> InfraResult<Vec<BalanceData>> {
        let body = json!({
            "ccy": assets,
        }).to_string();

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

        let all_balances: Vec<BalanceData> = bal_res
            .data
            .into_iter()
            .flat_map(|account| account.details)
            .map(BalanceData::from)
            .collect();

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

