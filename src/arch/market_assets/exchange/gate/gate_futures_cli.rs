use reqwest::Client;
use serde_json::json;
use simd_json::from_slice;
use std::sync::Arc;
use tracing::error;

use super::{
    api_key::{GateKey, read_gate_env_key},
    api_utils::*,
    config_assets::{
        GATE_BASE_URL, GATE_FUTURES_CONTRACT, GATE_FUTURES_CONTRACTS, GATE_FUTURES_FUNDING_RATE,
        GATE_FUTURES_WS_USDT, GATE_WS_FUTURES_CANDLES, GATE_WS_FUTURES_ORDERS,
        GATE_WS_FUTURES_TRADES,
    },
    gate_rest_msg::RestResGate,
    schemas::futures_rest::{
        contract_futures::RestContractGate, funding_rate::RestFundingRateGate,
    },
};
use crate::arch::{
    market_assets::{
        api_data::utils_data::{FundingRateData, InstrumentInfo},
        api_general::get_seconds_timestamp,
        base_data::{InstrumentType, SUBSCRIBE_LOWER, TRADING_LOWER},
    },
    task_execution::task_ws::{CandleParam, WsChannel},
    traits::{
        conversion::IntoInfraVec,
        market_lob::{LobPrivateRest, LobPublicRest, LobWebsocket, MarketLobApi},
    },
};
use crate::errors::{InfraError, InfraResult};

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

    pub fn ws_subscribe_private(&self, channel: &str, payload: Vec<String>) -> InfraResult<String> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?;

        let timestamp = get_seconds_timestamp();
        let auth = api_key.ws_auth(channel, SUBSCRIBE_LOWER, timestamp)?;

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

        let responds = self.client.get(url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: RestResGate<RestFundingRateGate> = from_slice(&mut res_bytes)?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(|entry| entry.into_funding_rate_data(inst))
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

        let responds = self.client.get(url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: RestResGate<RestContractGate> = from_slice(&mut res_bytes)?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(|entry| entry.into_funding_rate_data())
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
        let responds = self.client.get(url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: RestResGate<RestContractGate> = from_slice(&mut res_bytes)?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(|entry| entry.into_funding_rate_data())
            .collect();

        Ok(data)
    }

    async fn _get_futures_contracts(
        &self,
        settle: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> InfraResult<Vec<RestContractGate>> {
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

        let responds = self.client.get(url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: RestResGate<RestContractGate> = from_slice(&mut res_bytes)?;

        res.into_vec()
    }

    async fn _get_instrument_info(
        &self,
        inst_type: InstrumentType,
    ) -> InfraResult<Vec<InstrumentInfo>> {
        let mut data: Vec<InstrumentInfo> = Vec::new();

        if inst_type != InstrumentType::Perpetual {
            return Err(InfraError::ApiCliError(
                "Gate futures cli only supports perpetual instruments".into(),
            ));
        }

        for settle in ["usdt", "btc"] {
            let contracts = self._get_futures_contracts(settle, None, None).await?;
            data.extend(contracts.into_iter().map(InstrumentInfo::from));
        }

        Ok(data)
    }

    async fn _get_live_instruments(&self, inst_type: InstrumentType) -> InfraResult<Vec<String>> {
        let mut data: Vec<String> = Vec::new();

        if inst_type != InstrumentType::Perpetual {
            return Err(InfraError::ApiCliError(
                "Gate futures cli only supports perpetual instruments".into(),
            ));
        }

        for settle in ["usdt", "btc"] {
            let contracts = self._get_futures_contracts(settle, None, None).await?;
            data.extend(
                contracts
                    .into_iter()
                    .filter(|c| c.status.as_str() == TRADING_LOWER)
                    .map(|c| gate_inst_to_cli(&c.name)),
            );
        }

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
            WsChannel::Tick | WsChannel::Lob => Err(InfraError::Unimplemented),
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

    fn _get_private_sub_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        match channel {
            WsChannel::AccountOrders => {
                let api_key = self
                    .api_key
                    .as_ref()
                    .ok_or(InfraError::ApiCliNotInitialized)?;
                let payload = vec![api_key.user_id.clone(), "!all".into()];
                self.ws_subscribe_private(GATE_WS_FUTURES_ORDERS, payload)
            },
            _ => Err(InfraError::Unimplemented),
        }
    }
}
