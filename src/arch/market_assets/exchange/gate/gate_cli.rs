use reqwest::Client;
use simd_json::from_slice;
use std::sync::Arc;
use tracing::error;

use crate::arch::{
    market_assets::{
        api_data::{account_data::BalanceData, utils_data::FundingRateData},
        api_general::RequestMethod,
    },
    traits::{
        conversion::IntoInfraVec,
        market_lob::{LobPrivateRest, LobPublicRest, LobWebsocket, MarketLobApi},
    },
};
use crate::errors::{InfraError, InfraResult};

use super::{
    api_key::{GateKey, read_gate_env_key},
    api_utils::cli_perp_to_gate_inst,
    config_assets::*,
    gate_rest_msg::RestResGate,
    schemas::rest::{
        account_balance::RestAccountBalGate, contract::RestContractGate,
        funding_rate::RestFundingRateGate,
    },
};

#[derive(Clone, Debug)]
pub struct GateCli {
    pub client: Arc<Client>,
    pub api_key: Option<GateKey>,
}

impl Default for GateCli {
    fn default() -> Self {
        Self::new(Arc::new(Client::new()))
    }
}

impl MarketLobApi for GateCli {}

impl LobPublicRest for GateCli {}

impl LobPrivateRest for GateCli {
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

    async fn get_balance(&self, assets: Option<&[String]>) -> InfraResult<Vec<BalanceData>> {
        self._get_balance(assets).await
    }
}

impl LobWebsocket for GateCli {}

impl GateCli {
    pub fn new(shared_client: Arc<Client>) -> Self {
        Self {
            client: shared_client,
            api_key: None,
        }
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

    async fn _get_balance(&self, assets: Option<&[String]>) -> InfraResult<Vec<BalanceData>> {
        let res: RestResGate<RestAccountBalGate> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                None,
                None,
                GATE_BASE_URL,
                GATE_UNIFIED_ACCOUNTS,
            )
            .await?;

        let balances: Vec<BalanceData> = res
            .into_vec()?
            .into_iter()
            .flat_map(|account| account.into_balance_vec())
            .collect();

        let filtered = match assets {
            Some(list) if !list.is_empty() => balances
                .into_iter()
                .filter(|b| list.contains(&b.asset))
                .collect(),
            _ => balances,
        };

        Ok(filtered)
    }
}
